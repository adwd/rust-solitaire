/// Small original levels, ordered from an introduction to multi-crate puzzles.
/// `#` wall, `.` goal, `$` crate, `@` player, `*` crate on goal, `+` player on goal.
pub(crate) const LEVELS: &[(&str, &str)] = &[
    (
        "First delivery",
        "#######\n#     #\n# @$ .#\n#     #\n#######",
    ),
    (
        "Around the corner",
        "#######\n#   . #\n#   $ #\n# @   #\n#     #\n#######",
    ),
    (
        "Two deliveries",
        "########\n#      #\n# .  . #\n# $  $ #\n#   @  #\n#      #\n########",
    ),
    (
        "Center pillar",
        "########\n# .  . #\n#      #\n# $##$ #\n#      #\n#  @   #\n########",
    ),
    (
        "Side by side",
        "########\n#      #\n# ..   #\n# $$   #\n#  #   #\n# @    #\n#      #\n########",
    ),
    (
        "Loading bay",
        "#########\n#       #\n# ...   #\n# $ $ $ #\n#   @   #\n#       #\n#########",
    ),
    (
        "Through the gate",
        "#########\n#   . . #\n#       #\n### # ###\n# $   $ #\n#   @   #\n#       #\n#########",
    ),
    (
        "Warehouse",
        "#########\n# .   . #\n#   #   #\n# $   $ #\n#   *   #\n#  # #  #\n#   @   #\n#       #\n#########",
    ),
];
