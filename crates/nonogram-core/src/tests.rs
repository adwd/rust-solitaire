use super::*;

pub(super) fn candidates(size: usize, target: &[usize]) -> Vec<u16> {
    (0..1 << size)
        .filter(|mask| clues((0..size).map(|bit| mask & (1 << bit) != 0)) == target)
        .collect()
}

/// Solve from the visible clues alone, counting up to two solutions.
pub(super) fn solution_count(rows: &[Vec<u16>], columns: &[Vec<u16>], row: usize) -> usize {
    if row == rows.len() {
        return 1;
    }
    let mut count = 0;
    for mask in &rows[row] {
        let next: Vec<Vec<u16>> = columns
            .iter()
            .enumerate()
            .map(|(col, choices)| {
                choices
                    .iter()
                    .copied()
                    .filter(|choice| (choice >> row) & 1 == (mask >> col) & 1)
                    .collect()
            })
            .collect();
        if next.iter().all(|choices| !choices.is_empty()) {
            count += solution_count(rows, &next, row + 1);
        }
        if count >= 2 {
            return 2;
        }
    }
    count
}

#[test]
fn all_puzzles_have_exactly_one_solution_and_can_be_completed() {
    for (puzzle, (_, picture)) in puzzles::PUZZLES.iter().enumerate() {
        let mut game = Game::new(puzzle).unwrap();
        assert!(picture.iter().all(|row| row.len() == game.size()));
        let rows: Vec<_> = game
            .row_clues()
            .iter()
            .map(|clue| candidates(game.size(), clue))
            .collect();
        let cols: Vec<_> = game
            .column_clues()
            .iter()
            .map(|clue| candidates(game.size(), clue))
            .collect();
        assert_eq!(solution_count(&rows, &cols, 0), 1, "{}", game.name());
        assert!(!game.is_won());
        for row in 0..game.size() {
            for col in 0..game.size() {
                if game.solution[row * game.size() + col] {
                    assert!(game.apply(row, col, Cell::Filled));
                }
            }
        }
        assert!(game.is_won());
        assert!((0..game.size()).all(|i| game.row_complete(i) && game.column_complete(i)));
        assert!(!game.apply(0, 0, Cell::Crossed));
        assert!(game.undo());
        assert!(!game.is_won());
        assert!(game.redo());
        assert!(game.is_won());
    }
    assert!(Game::new(PUZZLE_COUNT).is_none());
}

#[test]
fn clues_include_empty_lines_and_separated_runs() {
    assert_eq!(clues([false; 5]), [0]);
    assert_eq!(clues([true; 5]), [5]);
    assert_eq!(clues([true, false, true, true, false, true]), [1, 2, 1]);
}

#[test]
fn marks_history_invalid_edits_and_restart() {
    let mut game = Game::default();
    assert!(game.apply(0, 0, Cell::Crossed));
    assert!(game.apply(0, 1, Cell::Filled));
    assert!(game.undo());
    let before = game.state.clone();
    assert!(!game.apply(0, 0, Cell::Crossed));
    assert!(!game.apply(usize::MAX, 1, Cell::Filled));
    assert!(!game.apply(1, game.size(), Cell::Filled));
    assert_eq!(game.state, before);
    assert!(game.redo());
    assert_eq!(game.cell(0, 1), Some(Cell::Filled));
    assert!(game.undo());
    assert!(game.apply(0, 0, Cell::Unknown));
    assert!(!game.can_redo());
    assert!(game.undo());
    assert!(game.undo());
    assert_eq!(game.moves(), 0);
    assert!(game.state.cells.iter().all(|&cell| cell == Cell::Unknown));
    assert!(!game.undo());
    game.restart();
    assert!(!game.can_undo() && !game.can_redo());
    assert_eq!(game.puzzle(), Some(0));
}

#[test]
fn wrong_filled_squares_prevent_completion() {
    let mut game = Game::default();
    assert!(game.apply(0, 0, Cell::Filled));
    for row in 0..game.size() {
        for col in 0..game.size() {
            if game.solution[row * game.size() + col] {
                game.apply(row, col, Cell::Filled);
            }
        }
    }
    assert!(!game.is_won());
    assert!(game.apply(0, 0, Cell::Crossed));
    assert!(game.is_won());
}
