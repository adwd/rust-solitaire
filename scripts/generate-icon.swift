// Draw the application icon from vector paths without external image assets.
import AppKit

guard CommandLine.arguments.count == 2 else {
    fatalError("Usage: generate-icon.swift <output.iconset>")
}
let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)

func color(_ red: CGFloat, _ green: CGFloat, _ blue: CGFloat) -> NSColor {
    NSColor(srgbRed: red / 255, green: green / 255, blue: blue / 255, alpha: 1)
}

func drawIcon() {
    color(16, 65, 55).setFill()
    NSBezierPath(roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896),
                 xRadius: 195, yRadius: 195).fill()

    // A blue card back behind the face-up card.
    NSGraphicsContext.saveGraphicsState()
    let tilt = NSAffineTransform()
    tilt.translateX(by: 495, yBy: 512)
    tilt.rotate(byDegrees: 12)
    tilt.concat()
    color(255, 250, 239).setFill()
    NSBezierPath(roundedRect: NSRect(x: -285, y: -325, width: 465, height: 650),
                 xRadius: 35, yRadius: 35).fill()
    color(34, 75, 88).setFill()
    NSBezierPath(roundedRect: NSRect(x: -264, y: -304, width: 423, height: 608),
                 xRadius: 20, yRadius: 20).fill()
    color(118, 158, 162).setStroke()
    let inner = NSBezierPath(roundedRect: NSRect(x: -245, y: -285, width: 385, height: 570),
                            xRadius: 15, yRadius: 15)
    inner.lineWidth = 5
    inner.stroke()
    NSGraphicsContext.restoreGraphicsState()

    // Cream Ace card, with rank labels and one central heart.
    NSGraphicsContext.saveGraphicsState()
    let face = NSAffineTransform()
    face.translateX(by: 569, yBy: 512)
    face.rotate(byDegrees: -9)
    face.concat()
    NSColor.black.withAlphaComponent(0.20).setFill()
    NSBezierPath(roundedRect: NSRect(x: -225, y: -337, width: 465, height: 650),
                 xRadius: 35, yRadius: 35).fill()
    color(255, 250, 239).setFill()
    NSBezierPath(roundedRect: NSRect(x: -232.5, y: -325, width: 465, height: 650),
                 xRadius: 35, yRadius: 35).fill()
    let ink = color(183, 48, 59)
    ink.setFill()
    let heart = NSBezierPath()
    heart.move(to: NSPoint(x: 0, y: -133))
    heart.curve(to: NSPoint(x: -137, y: 58),
                controlPoint1: NSPoint(x: -45, y: -77),
                controlPoint2: NSPoint(x: -137, y: -8))
    heart.curve(to: NSPoint(x: 0, y: 103),
                controlPoint1: NSPoint(x: -137, y: 154),
                controlPoint2: NSPoint(x: -40, y: 156))
    heart.curve(to: NSPoint(x: 137, y: 58),
                controlPoint1: NSPoint(x: 40, y: 156),
                controlPoint2: NSPoint(x: 137, y: 154))
    heart.curve(to: NSPoint(x: 0, y: -133),
                controlPoint1: NSPoint(x: 137, y: -8),
                controlPoint2: NSPoint(x: 45, y: -77))
    heart.close()
    heart.fill()
    let rank = NSString(string: "A")
    let attributes: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: 85, weight: .medium),
        .foregroundColor: ink,
    ]
    rank.draw(at: NSPoint(x: -190, y: 211), withAttributes: attributes)
    let rankSize = rank.size(withAttributes: attributes)
    rank.draw(at: NSPoint(x: 190 - rankSize.width, y: -292), withAttributes: attributes)
    NSGraphicsContext.restoreGraphicsState()
}

let representations: [(String, Int)] = [
    ("icon_16x16.png", 16), ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32), ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128), ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256), ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512), ("icon_512x512@2x.png", 1024),
]
for (name, size) in representations {
    guard let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
        isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0
    ), let context = NSGraphicsContext(bitmapImageRep: bitmap) else {
        fatalError("Cannot create icon bitmap")
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    context.shouldAntialias = true
    let scale = NSAffineTransform()
    scale.scale(by: CGFloat(size) / 1024)
    scale.concat()
    drawIcon()
    NSGraphicsContext.restoreGraphicsState()
    guard let png = bitmap.representation(using: .png, properties: [:]) else {
        fatalError("Cannot encode icon PNG")
    }
    try png.write(to: output.appendingPathComponent(name), options: .atomic)
}
