// Draws Lumen's app icons: a ring of light around a check. Run from the app:
// swift tools/icons.swift
import AppKit

let size = 1024
func color(_ hex: UInt32, _ a: CGFloat = 1) -> CGColor {
    CGColor(srgbRed: CGFloat((hex >> 16) & 255) / 255, green: CGFloat((hex >> 8) & 255) / 255, blue: CGFloat(hex & 255) / 255, alpha: a)
}
struct Style { let name: String; let back: [UInt32]; let glow: UInt32; let ring: [UInt32]; let check: UInt32 }
let styles = [
    Style(name: "aurora", back: [0x1B1140, 0x4B2FD6, 0x8D6BFF], glow: 0xFF6FA8, ring: [0xFFFFFF, 0xD9CEFF], check: 0xFFFFFF),
    Style(name: "midnight", back: [0x05040D, 0x0F0B26, 0x1C1640], glow: 0x7C5CFF, ring: [0x9D8BFF, 0x5EE6C4], check: 0xE9E4FF),
    Style(name: "sunrise", back: [0xFF6F61, 0xFF9A5A, 0xFFD27A], glow: 0xFFFFFF, ring: [0xFFFFFF, 0xFFF1DC], check: 0xFFFFFF),
    Style(name: "mono", back: [0xF7F7F5, 0xEDEDEA, 0xDEDEDA], glow: 0xFFFFFF, ring: [0x1C1C1E, 0x48484A], check: 0x1C1C1E),
]
let space = CGColorSpace(name: CGColorSpace.sRGB)!
for style in styles {
    // Drawn at three times the size and scaled down, so every edge is smooth.
    let ctx = CGContext(data: nil, width: size * 3, height: size * 3, bitsPerComponent: 8, bytesPerRow: 0, space: space, bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
    ctx.scaleBy(x: 3, y: 3)
    let s = CGFloat(size)
    let back = CGGradient(colorsSpace: space, colors: style.back.map { color($0) } as CFArray, locations: [0, 0.55, 1])!
    ctx.drawLinearGradient(back, start: CGPoint(x: 0, y: 0), end: CGPoint(x: s, y: s), options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
    let glow = CGGradient(colorsSpace: space, colors: [color(style.glow, 0.75), color(style.glow, 0)] as CFArray, locations: [0, 1])!
    ctx.drawRadialGradient(glow, startCenter: CGPoint(x: s * 0.78, y: s * 0.2), startRadius: 0, endCenter: CGPoint(x: s * 0.78, y: s * 0.2), endRadius: s * 0.62, options: [])
    ctx.drawRadialGradient(glow, startCenter: CGPoint(x: s * 0.12, y: s * 0.9), startRadius: 0, endCenter: CGPoint(x: s * 0.12, y: s * 0.9), endRadius: s * 0.4, options: [])
    // The ring: an arc of light fading toward its tail.
    let center = CGPoint(x: s / 2, y: s / 2), radius = s * 0.29
    let head = CGFloat.pi * 0.36, sweep = CGFloat.pi * 1.82
    let arc = CGMutablePath()
    arc.addArc(center: center, radius: radius, startAngle: head, endAngle: head - sweep, clockwise: true)
    ctx.saveGState()
    ctx.addPath(arc.copy(strokingWithWidth: s * 0.066, lineCap: .round, lineJoin: .round, miterLimit: 1))
    ctx.clip()
    // Wedges laid edge to edge without antialiasing; the clip smooths the rim.
    ctx.setShouldAntialias(false)
    let steps = 720, margin = CGFloat(0.14)
    for i in 0..<steps {
        let a0 = head + margin - (.pi * 2) * CGFloat(i) / CGFloat(steps)
        let a1 = head + margin - (.pi * 2) * CGFloat(i + 1) / CGFloat(steps)
        // 0 at the head, 1 at the tail.
        let t = min(1, max(0, (head - a0) / sweep))
        func ch(_ v: UInt32, _ shift: UInt32) -> CGFloat { CGFloat((v >> shift) & 255) / 255 }
        let a = style.ring[0], b = style.ring[1]
        let alpha = a0 < head - sweep - 0.06 ? 1 - 0.86 * 1 : 1 - 0.86 * t
        ctx.setFillColor(CGColor(srgbRed: ch(a, 16) + (ch(b, 16) - ch(a, 16)) * t, green: ch(a, 8) + (ch(b, 8) - ch(a, 8)) * t, blue: ch(a, 0) + (ch(b, 0) - ch(a, 0)) * t, alpha: alpha))
        ctx.move(to: center)
        ctx.addLine(to: CGPoint(x: center.x + s * cos(a0), y: center.y + s * sin(a0)))
        ctx.addLine(to: CGPoint(x: center.x + s * cos(a1), y: center.y + s * sin(a1)))
        ctx.closePath()
        ctx.fillPath()
    }
    ctx.restoreGState()
    ctx.setShouldAntialias(true)
    ctx.setShadow(offset: CGSize(width: 0, height: -14), blur: 34, color: color(0x000000, 0.22))
    ctx.setStrokeColor(color(style.check))
    ctx.setLineWidth(s * 0.082)
    ctx.setLineCap(.round)
    ctx.setLineJoin(.round)
    ctx.move(to: CGPoint(x: s * 0.385, y: s * 0.505))
    ctx.addLine(to: CGPoint(x: s * 0.475, y: s * 0.415))
    ctx.addLine(to: CGPoint(x: s * 0.642, y: s * 0.6))
    ctx.strokePath()
    // A spark at the ring's head.
    ctx.setShadow(offset: .zero, blur: 40, color: color(style.glow, 0.9))
    ctx.setFillColor(color(style.ring[0] == 0x1C1C1E ? 0x1C1C1E : 0xFFFFFF))
    let tip = CGPoint(x: center.x + radius * cos(head), y: center.y + radius * sin(head))
    ctx.fillEllipse(in: CGRect(x: tip.x - 30, y: tip.y - 30, width: 60, height: 60))
    let large = ctx.makeImage()!
    let small = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: 0, space: space, bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
    small.interpolationQuality = .high
    small.draw(large, in: CGRect(x: 0, y: 0, width: size, height: size))
    let image = small.makeImage()!
    let rep = NSBitmapImageRep(cgImage: image)
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "assets/icons/\(style.name).png"))
    print("assets/icons/\(style.name).png")
}
