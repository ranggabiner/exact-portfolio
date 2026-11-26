// Lumen's native views, in Apple's own frameworks:
//   lumen-aurora    MeshGradient (iOS 18) that drifts and leans with Core Motion
//   lumen-hero      floating glass cards with parallax, for the welcome
//   lumen-physics   SpriteKit task bubbles that fall with the phone's gravity
//   lumen-confetti  a Core Animation burst with a Core Haptics pattern
//   lumen-ring      a progress ring with spring motion and numeric transitions
// Every effect stills under Reduce Motion and Low Power Mode, and sensors run
// only while a view that reads them is on screen.
import CoreHaptics
import CoreMotion
import SpriteKit
import SwiftUI
import UIKit

final class LumenModule: ExactModule {
    override class var views: [String: ExactNativeFactory] {
        [
            "lumen-aurora": ExactNativeFactory(firstFrame: true) { props, events in AuroraInstance(props: props, events: events) },
            "lumen-hero": ExactNativeFactory { props, events in HeroInstance(props: props, events: events) },
            "lumen-physics": ExactNativeFactory { props, events in PhysicsInstance(props: props, events: events) },
            "lumen-confetti": ExactNativeFactory { props, events in ConfettiInstance(props: props, events: events) },
            "lumen-ring": ExactNativeFactory(firstFrame: true) { props, events in RingInstance(props: props, events: events) },
        ]
    }
}

let exactModule: ExactModule.Type = LumenModule.self

// MARK: - Shared

private extension Color {
    init(hex: UInt32, opacity: Double = 1) {
        self.init(.sRGB, red: Double((hex >> 16) & 255) / 255, green: Double((hex >> 8) & 255) / 255, blue: Double(hex & 255) / 255, opacity: opacity)
    }
}

private extension UIColor {
    convenience init(hex: UInt32, alpha: CGFloat = 1) {
        self.init(red: CGFloat((hex >> 16) & 255) / 255, green: CGFloat((hex >> 8) & 255) / 255, blue: CGFloat(hex & 255) / 255, alpha: alpha)
    }
}

@MainActor private var calmDown: Bool {
    UIAccessibility.isReduceMotionEnabled || ProcessInfo.processInfo.isLowPowerModeEnabled
}

/// One motion manager for the app, started by the first view that watches and
/// stopped when the last one leaves. Device motion asks no permission.
@MainActor final class Tilt: ObservableObject {
    static let shared = Tilt()
    @Published var x: Double = 0
    @Published var y: Double = 0
    private(set) var gravity = CGVector(dx: 0, dy: -1)
    private let manager = CMMotionManager()
    private var watchers = 0

    func start() {
        watchers += 1
        guard watchers == 1, manager.isDeviceMotionAvailable, !calmDown else { return }
        manager.deviceMotionUpdateInterval = 1.0 / 30.0
        manager.startDeviceMotionUpdates(to: .main) { [weak self] motion, _ in
            guard let self, let g = motion?.gravity else { return }
            MainActor.assumeIsolated {
                self.gravity = CGVector(dx: g.x, dy: g.y)
                // Eased, so a shaky hand reads as a slow lean.
                self.x += (max(-1, min(1, g.x * 1.6)) - self.x) * 0.12
                self.y += (max(-1, min(1, (g.y + 0.7) * 1.6)) - self.y) * 0.12
            }
        }
    }

    func stop() {
        watchers = max(0, watchers - 1)
        if watchers == 0 { manager.stopDeviceMotionUpdates() }
    }
}

/// Core Haptics patterns for the moments that deserve more than a tap.
@MainActor final class Feel {
    static let shared = Feel()
    private var engine: CHHapticEngine?

    private func ready() -> CHHapticEngine? {
        guard CHHapticEngine.capabilitiesForHardware().supportsHaptics else { return nil }
        if engine == nil {
            engine = try? CHHapticEngine()
            engine?.isAutoShutdownEnabled = true
            engine?.resetHandler = { [weak self] in Task { @MainActor in self?.engine = nil } }
        }
        try? engine?.start()
        return engine
    }

    private func tap(_ time: TimeInterval, _ intensity: Float, _ sharpness: Float) -> CHHapticEvent {
        CHHapticEvent(eventType: .hapticTransient, parameters: [
            CHHapticEventParameter(parameterID: .hapticIntensity, value: intensity),
            CHHapticEventParameter(parameterID: .hapticSharpness, value: sharpness),
        ], relativeTime: time)
    }

    private func swell(_ time: TimeInterval, _ duration: TimeInterval, _ intensity: Float) -> CHHapticEvent {
        CHHapticEvent(eventType: .hapticContinuous, parameters: [
            CHHapticEventParameter(parameterID: .hapticIntensity, value: intensity),
            CHHapticEventParameter(parameterID: .hapticSharpness, value: 0.2),
            CHHapticEventParameter(parameterID: .attackTime, value: 0.2),
            CHHapticEventParameter(parameterID: .releaseTime, value: 0.35),
        ], relativeTime: time, duration: duration)
    }

    func play(_ name: String) {
        guard let engine = ready() else { return }
        let events: [CHHapticEvent]
        switch name {
        case "confetti":
            events = [swell(0, 0.25, 0.45), tap(0.04, 0.7, 0.5), tap(0.14, 0.85, 0.7), tap(0.26, 1, 0.9), tap(0.42, 0.5, 0.6), tap(0.55, 0.35, 0.8)]
        case "calm":
            events = [swell(0, 0.7, 0.4), tap(0.55, 0.35, 0.2)]
        case "pop":
            events = [tap(0, 0.65, 0.85)]
        default:
            events = [tap(0, 0.8, 0.4), tap(0.12, 1, 0.8)]
        }
        guard let pattern = try? CHHapticPattern(events: events, parameters: []),
              let player = try? engine.makePlayer(with: pattern) else { return }
        try? player.start(atTime: CHHapticTimeImmediate)
    }
}

/// A SwiftUI view in a node's box. It takes no touches and no safe area.
@MainActor private func decorative<V: View>(_ view: V) -> UIHostingController<V> {
    let host = UIHostingController(rootView: view)
    host.view.backgroundColor = .clear
    host.view.isUserInteractionEnabled = false
    host.safeAreaRegions = []
    return host
}

// MARK: - Aurora

@MainActor private final class AuroraModel: ObservableObject {
    @Published var palette = "dawn"
}

private struct AuroraPalette {
    let base: UInt32
    let colors: [UInt32]
    static func named(_ name: String) -> AuroraPalette {
        switch name {
        case "dusk": return .init(base: 0x0B0820, colors: [0x2A1670, 0x6A2E8C, 0xB0417A])
        case "violet": return .init(base: 0x0B0820, colors: [0x2D1B8F, 0x6C4BE8, 0x3A2BB0])
        case "calm": return .init(base: 0x07121F, colors: [0x0F3A63, 0x1D6F86, 0x3B2F9A])
        case "rose": return .init(base: 0x140822, colors: [0x5A1E6E, 0xB83C78, 0x4B2FD6])
        case "focus": return .init(base: 0x070617, colors: [0x241670, 0x4B2FD6, 0x14507A])
        case "celebrate": return .init(base: 0x120A26, colors: [0x7C5CFF, 0xFF6FA8, 0xFFB457])
        default: return .init(base: 0x0B0820, colors: [0x3A22A8, 0x8D5BEA, 0xE0609A])
        }
    }
}

private struct AuroraView: View {
    @ObservedObject var model: AuroraModel
    @ObservedObject var tilt = Tilt.shared
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 30.0, paused: reduceMotion || ProcessInfo.processInfo.isLowPowerModeEnabled)) { timeline in
            let time = reduceMotion ? 0 : timeline.date.timeIntervalSinceReferenceDate
            layer(time: time)
        }
        .animation(.easeInOut(duration: 1.1), value: model.palette)
        .ignoresSafeArea()
        .onAppear { tilt.start() }
        .onDisappear { tilt.stop() }
    }

    @ViewBuilder private func layer(time: TimeInterval) -> some View {
        let p = AuroraPalette.named(model.palette)
        if #available(iOS 18.0, *) {
            let s = Float(sin(time * 0.35)), c = Float(cos(time * 0.27)), w = Float(sin(time * 0.19 + 1.7))
            let tx = Float(tilt.x) * 0.12, ty = Float(tilt.y) * 0.08
            MeshGradient(
                width: 3, height: 4,
                points: [
                    [0, 0], [0.5 + 0.12 * s, 0], [1, 0],
                    [0, 0.28 + 0.07 * c], [0.45 + 0.22 * c + tx, 0.30 + 0.09 * s - ty], [1, 0.24 + 0.08 * w],
                    [0, 0.66 + 0.07 * w], [0.58 + 0.2 * w - tx, 0.64 + 0.1 * c + ty], [1, 0.70 + 0.06 * s],
                    [0, 1], [0.5 + 0.1 * c, 1], [1, 1],
                ],
                colors: [
                    Color(hex: p.colors[0]), Color(hex: p.colors[1]), Color(hex: p.base),
                    Color(hex: p.base), Color(hex: p.colors[1], opacity: 0.92), Color(hex: p.colors[2]),
                    Color(hex: p.colors[0], opacity: 0.7), Color(hex: p.base), Color(hex: p.base),
                    Color(hex: p.base), Color(hex: p.base), Color(hex: p.colors[0], opacity: 0.6),
                ],
                smoothsColors: true
            )
            .overlay(shade(p))
        } else {
            GeometryReader { g in
                ZStack {
                    Color(hex: p.base)
                    ForEach(0..<3, id: \.self) { i in
                        let phase = time * (0.2 + Double(i) * 0.07) + Double(i) * 2.1
                        Circle()
                            .fill(Color(hex: p.colors[i], opacity: 0.75))
                            .frame(width: g.size.width * 0.95)
                            .blur(radius: 90)
                            .offset(
                                x: CGFloat(sin(phase)) * g.size.width * 0.35 + CGFloat(tilt.x) * 24,
                                y: CGFloat(i - 1) * g.size.height * 0.3 + CGFloat(cos(phase)) * 50 + CGFloat(tilt.y) * 18
                            )
                    }
                    shade(p)
                }
            }
        }
    }

    /// Darkens the lower part so white text always reads.
    private func shade(_ p: AuroraPalette) -> some View {
        LinearGradient(colors: [Color(hex: p.base, opacity: 0.05), Color(hex: p.base, opacity: 0.42), Color(hex: p.base, opacity: 0.72)], startPoint: .top, endPoint: .bottom)
    }
}

final class AuroraInstance: ExactNativeInstance {
    private var host: UIViewController!
    private var model: AnyObject!

    init(props: [String: String], events: ExactNativeEvents) {
        super.init(events: events)
        MainActor.assumeIsolated {
            let model = AuroraModel()
            model.palette = props["palette"] ?? "dawn"
            self.model = model
            self.host = decorative(AuroraView(model: model))
        }
    }

    override var view: ExactNativeView { MainActor.assumeIsolated { host.view } }
    override var controller: UIViewController? { host }
    override func setProps(_ props: [String: String]) throws {
        MainActor.assumeIsolated { (model as? AuroraModel)?.palette = props["palette"] ?? "dawn" }
    }
}

// MARK: - Hero

@MainActor private final class HeroModel: ObservableObject {
    @Published var labels: [String] = []
}

private struct HeroView: View {
    @ObservedObject var model: HeroModel
    @ObservedObject var tilt = Tilt.shared
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var arrived = false
    @State private var finished = 0

    private let symbols = ["phone.fill", "paperplane.fill", "figure.run", "creditcard.fill"]
    private let tints: [UInt32] = [0xFF6FA8, 0x8D6BFF, 0x37BFA4, 0xFFB457]
    private let places: [(x: CGFloat, y: CGFloat, depth: CGFloat, angle: Double)] = [
        (-72, -96, 22, -7), (70, -38, 38, 5), (-56, 46, 30, 4), (62, 104, 16, -5),
    ]

    var body: some View {
        ZStack {
            Circle()
                .fill(RadialGradient(colors: [Color(hex: 0xB39BFF, opacity: 0.75), Color(hex: 0x7C5CFF, opacity: 0.0)], center: .center, startRadius: 4, endRadius: 150))
                .frame(width: 300, height: 300)
                .scaleEffect(arrived ? 1 : 0.5)
                .offset(x: CGFloat(tilt.x) * -10, y: CGFloat(tilt.y) * -8)
            ForEach(Array(model.labels.prefix(4).enumerated()), id: \.offset) { index, label in
                card(index: index, label: label)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onAppear {
            tilt.start()
            withAnimation(.spring(response: 0.9, dampingFraction: 0.68).delay(0.15)) { arrived = true }
        }
        .onDisappear { tilt.stop() }
        .task {
            // The cards tick themselves off in turn, then start over.
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 1_300_000_000)
                withAnimation(.spring(response: 0.45, dampingFraction: 0.6)) { finished = (finished + 1) % 6 }
            }
        }
    }

    private func card(index: Int, label: String) -> some View {
        let place = places[index % places.count]
        let done = index < finished
        return TimelineView(.animation(minimumInterval: 1.0 / 30.0, paused: reduceMotion)) { timeline in
            let t = reduceMotion ? 0 : timeline.date.timeIntervalSinceReferenceDate
            let bob = CGFloat(sin(t * 1.1 + Double(index) * 1.6)) * 5
            HStack(spacing: 10) {
                Image(systemName: symbols[index % symbols.count])
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(.white)
                    .frame(width: 32, height: 32)
                    .background(Color(hex: tints[index % tints.count]), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                Text(label)
                    .font(.system(size: 15, weight: .semibold, design: .rounded))
                    .foregroundStyle(.white)
                    .strikethrough(done, color: .white.opacity(0.7))
                    .lineLimit(1)
                Image(systemName: done ? "checkmark.circle.fill" : "circle")
                    .font(.system(size: 19, weight: .semibold))
                    .foregroundStyle(done ? Color(hex: 0x5EE6C4) : .white.opacity(0.55))
                    .contentTransition(.symbolEffect(.replace))
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 11)
            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 18, style: .continuous).strokeBorder(LinearGradient(colors: [.white.opacity(0.45), .white.opacity(0.05)], startPoint: .topLeading, endPoint: .bottomTrailing), lineWidth: 1))
            .environment(\.colorScheme, .dark)
            .shadow(color: .black.opacity(0.35), radius: 18, y: 12)
            .rotationEffect(.degrees(place.angle))
            .rotation3DEffect(.degrees(Double(tilt.x) * 12), axis: (x: 0, y: 1, z: 0))
            .rotation3DEffect(.degrees(Double(tilt.y) * -9), axis: (x: 1, y: 0, z: 0))
            .offset(
                x: arrived ? place.x + CGFloat(tilt.x) * place.depth : 0,
                y: arrived ? place.y + bob + CGFloat(tilt.y) * place.depth * 0.7 : 40
            )
            .scaleEffect(arrived ? 1 : 0.6)
            .opacity(arrived ? 1 : 0)
        }
    }
}

final class HeroInstance: ExactNativeInstance {
    private var host: UIViewController!
    private var model: AnyObject!

    init(props: [String: String], events: ExactNativeEvents) {
        super.init(events: events)
        MainActor.assumeIsolated {
            let model = HeroModel()
            model.labels = (props["labels"] ?? "").split(separator: "|").map(String.init)
            self.model = model
            self.host = decorative(HeroView(model: model))
        }
    }

    override var view: ExactNativeView { MainActor.assumeIsolated { host.view } }
    override var controller: UIViewController? { host }
    override func setProps(_ props: [String: String]) throws {
        MainActor.assumeIsolated { (model as? HeroModel)?.labels = (props["labels"] ?? "").split(separator: "|").map(String.init) }
    }
}

// MARK: - Physics

private final class BubbleScene: SKScene {
    var items: [String] = []
    private var built = CGSize.zero
    private let tints: [UInt32] = [0x7C5CFF, 0xB83C78, 0x1D8F86, 0xC9772B, 0x4B2FD6]

    override func didMove(to view: SKView) { rebuild() }
    override func didChangeSize(_ oldSize: CGSize) { rebuild() }

    func rebuild(force: Bool = false) {
        guard size.width > 40, size.height > 40, force || built != size else { return }
        built = size
        removeAllChildren()
        removeAllActions()
        physicsBody = SKPhysicsBody(edgeLoopFrom: CGRect(origin: .zero, size: size))
        physicsWorld.gravity = CGVector(dx: 0, dy: -3.2)
        for (index, item) in items.enumerated() {
            run(.sequence([.wait(forDuration: 0.18 * Double(index)), .run { [weak self] in self?.drop(item, index: index) }]))
        }
    }

    private func drop(_ item: String, index: Int) {
        // SpriteKit cannot name the system font, so the text carries it.
        let base = UIFont.systemFont(ofSize: 15, weight: .semibold)
        let font = base.fontDescriptor.withDesign(.rounded).map { UIFont(descriptor: $0, size: 15) } ?? base
        let label = SKLabelNode()
        label.attributedText = NSAttributedString(string: item, attributes: [.font: font, .foregroundColor: UIColor.white])
        label.verticalAlignmentMode = .center
        let width = min(size.width - 24, label.frame.width + 34), height: CGFloat = 40
        let path = CGPath(roundedRect: CGRect(x: -width / 2, y: -height / 2, width: width, height: height), cornerWidth: height / 2, cornerHeight: height / 2, transform: nil)
        let bubble = SKShapeNode(path: path)
        bubble.name = "bubble"
        bubble.fillColor = UIColor(hex: tints[index % tints.count], alpha: 0.92)
        bubble.strokeColor = UIColor.white.withAlphaComponent(0.35)
        bubble.lineWidth = 1
        bubble.addChild(label)
        bubble.position = CGPoint(x: CGFloat.random(in: width / 2 ... max(width / 2, size.width - width / 2)), y: size.height - height)
        bubble.zRotation = CGFloat.random(in: -0.3 ... 0.3)
        let body = SKPhysicsBody(polygonFrom: path)
        body.restitution = 0.45
        body.friction = 0.35
        body.linearDamping = 0.35
        body.angularDamping = 0.4
        bubble.physicsBody = body
        bubble.setScale(0.4)
        addChild(bubble)
        bubble.run(.scale(to: 1, duration: 0.22))
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let point = touches.first?.location(in: self) else { return }
        guard let hit = nodes(at: point).compactMap({ $0.name == "bubble" ? $0 : $0.parent?.name == "bubble" ? $0.parent : nil }).first else {
            for node in children where node.name == "bubble" {
                node.physicsBody?.applyImpulse(CGVector(dx: CGFloat.random(in: -6 ... 6), dy: CGFloat.random(in: 8 ... 22)))
            }
            return
        }
        hit.name = nil
        hit.physicsBody = nil
        MainActor.assumeIsolated { Feel.shared.play("pop") }
        hit.run(.sequence([.group([.scale(to: 1.35, duration: 0.18), .fadeOut(withDuration: 0.18)]), .removeFromParent()]))
    }

    override func update(_ currentTime: TimeInterval) {
        MainActor.assumeIsolated {
            let g = Tilt.shared.gravity
            if !calmDown { physicsWorld.gravity = CGVector(dx: g.dx * 6, dy: min(-1.2, g.dy * 4.5)) }
        }
    }
}

private final class BubbleView: SKView {
    let bubbles = BubbleScene(size: .zero)
    private var watching = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear
        allowsTransparency = true
        preferredFramesPerSecond = 60
        bubbles.scaleMode = .resizeFill
        bubbles.backgroundColor = .clear
        presentScene(bubbles)
    }

    required init?(coder: NSCoder) { nil }

    override func layoutSubviews() {
        super.layoutSubviews()
        if bubbles.size != bounds.size { bubbles.size = bounds.size }
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        let visible = window != nil
        isPaused = !visible
        guard visible != watching else { return }
        watching = visible
        if visible { Tilt.shared.start() } else { Tilt.shared.stop() }
    }
}

final class PhysicsInstance: ExactNativeInstance {
    private var bubbles: UIView!

    init(props: [String: String], events: ExactNativeEvents) {
        super.init(events: events)
        MainActor.assumeIsolated {
            let view = BubbleView(frame: .zero)
            view.bubbles.items = (props["items"] ?? "").split(separator: "|").map(String.init)
            self.bubbles = view
        }
    }

    override var view: ExactNativeView { bubbles }
    override func setProps(_ props: [String: String]) throws {
        MainActor.assumeIsolated {
            guard let view = bubbles as? BubbleView else { return }
            let items = (props["items"] ?? "").split(separator: "|").map(String.init)
            if items != view.bubbles.items {
                view.bubbles.items = items
                view.bubbles.rebuild(force: true)
            }
        }
    }
}

// MARK: - Confetti

private final class ConfettiView: UIView {
    private var pending: String?

    func burst(_ kind: String) {
        guard bounds.width > 0, window != nil else { pending = kind; return }
        Feel.shared.play(kind == "confetti" ? "confetti" : "calm")
        guard kind != "haptic" else { return }
        if kind == "calm" || calmDown { glow() } else { rain() }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        if let kind = pending, bounds.width > 0, window != nil {
            pending = nil
            burst(kind)
        }
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        setNeedsLayout()
    }

    private func piece(_ size: CGSize, round: Bool) -> CGImage? {
        UIGraphicsImageRenderer(size: size).image { context in
            UIColor.white.setFill()
            if round {
                context.cgContext.fillEllipse(in: CGRect(origin: .zero, size: size))
            } else {
                UIBezierPath(roundedRect: CGRect(origin: .zero, size: size), cornerRadius: 2).fill()
            }
        }.cgImage
    }

    private func rain() {
        let emitter = CAEmitterLayer()
        emitter.emitterPosition = CGPoint(x: bounds.midX, y: -12)
        emitter.emitterShape = .line
        emitter.emitterSize = CGSize(width: bounds.width * 0.9, height: 1)
        let colors: [UInt32] = [0x8D6BFF, 0xFF6FA8, 0xFFB457, 0x5EE6C4, 0x5AA9FF, 0xFFFFFF]
        let shapes = [piece(CGSize(width: 10, height: 14), round: false), piece(CGSize(width: 9, height: 9), round: true), piece(CGSize(width: 5, height: 18), round: false)]
        emitter.emitterCells = colors.enumerated().map { index, color in
            let cell = CAEmitterCell()
            cell.contents = shapes[index % shapes.count]
            cell.color = UIColor(hex: color).cgColor
            cell.birthRate = 28
            cell.lifetime = 4.2
            cell.velocity = 260
            cell.velocityRange = 140
            cell.emissionLongitude = .pi
            cell.emissionRange = .pi / 3
            cell.yAcceleration = 240
            cell.xAcceleration = CGFloat(index % 2 == 0 ? 14 : -14)
            cell.spin = 3.5
            cell.spinRange = 7
            cell.scale = 0.75
            cell.scaleRange = 0.35
            cell.alphaSpeed = -0.22
            return cell
        }
        emitter.beginTime = CACurrentMediaTime()
        layer.addSublayer(emitter)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) { emitter.birthRate = 0 }
        DispatchQueue.main.asyncAfter(deadline: .now() + 4.8) { emitter.removeFromSuperlayer() }
    }

    private func glow() {
        let light = CAGradientLayer()
        light.type = .radial
        light.colors = [UIColor(hex: 0x9D8BFF, alpha: 0.55).cgColor, UIColor(hex: 0xFF6FA8, alpha: 0.18).cgColor, UIColor.clear.cgColor]
        light.locations = [0, 0.45, 1]
        light.startPoint = CGPoint(x: 0.5, y: 0.5)
        light.endPoint = CGPoint(x: 1.1, y: 1.1)
        light.frame = bounds
        light.opacity = 0
        layer.addSublayer(light)
        let breathe = CAKeyframeAnimation(keyPath: "opacity")
        breathe.values = [0, 1, 0]
        breathe.keyTimes = [0, 0.35, 1]
        breathe.duration = 1.6
        light.add(breathe, forKey: "glow")
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.7) { light.removeFromSuperlayer() }
    }
}

final class ConfettiInstance: ExactNativeInstance {
    private var confetti: UIView!
    private var last = "0"

    init(props: [String: String], events: ExactNativeEvents) {
        super.init(events: events)
        MainActor.assumeIsolated {
            let view = ConfettiView(frame: .zero)
            view.isUserInteractionEnabled = false
            view.backgroundColor = .clear
            self.confetti = view
        }
    }

    override var view: ExactNativeView { confetti }
    override func setProps(_ props: [String: String]) throws {
        let burst = props["burst"] ?? "0"
        guard burst != last, burst != "0" else { return }
        last = burst
        let kind = props["kind"] ?? "confetti"
        MainActor.assumeIsolated { (confetti as? ConfettiView)?.burst(kind) }
    }
}

// MARK: - Ring

@MainActor private final class RingModel: ObservableObject {
    @Published var progress: Double = 0
    @Published var label = ""
    @Published var caption = ""
    @Published var thick = false
    func apply(_ props: [String: String]) {
        let value = max(0, min(1, Double(props["progress"] ?? "0") ?? 0))
        if value != progress { progress = value }
        label = props["figure"] ?? ""
        caption = props["suffix"] ?? ""
        thick = props["thick"] == "1"
    }
}

private struct RingView: View {
    @ObservedObject var model: RingModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var drawn: Double = 0

    var body: some View {
        GeometryReader { g in
            let side = min(g.size.width, g.size.height)
            let width = model.thick ? side * 0.055 : side * 0.12
            ZStack {
                Circle().stroke(Color(hex: 0x7C5CFF, opacity: 0.18), lineWidth: width)
                Circle()
                    .trim(from: 0, to: max(0.001, drawn))
                    .stroke(
                        AngularGradient(colors: [Color(hex: 0x6A45F0), Color(hex: 0x9D8BFF), Color(hex: 0xFF6FA8), Color(hex: 0x6A45F0)], center: .center),
                        style: StrokeStyle(lineWidth: width, lineCap: .round)
                    )
                    .rotationEffect(.degrees(-90))
                    .shadow(color: Color(hex: 0x8D6BFF, opacity: drawn >= 0.999 ? 0.9 : 0.35), radius: drawn >= 0.999 ? 12 : 5)
                    .opacity(model.progress <= 0 ? 0 : 1)
                if !model.label.isEmpty {
                    HStack(alignment: .firstTextBaseline, spacing: 1) {
                        Text(model.label)
                            .font(.system(size: side * 0.32, weight: .heavy, design: .rounded))
                            .contentTransition(.numericText())
                        Text(model.caption)
                            .font(.system(size: side * 0.15, weight: .semibold, design: .rounded))
                            .foregroundStyle(.secondary)
                            .contentTransition(.numericText())
                    }
                    .monospacedDigit()
                    .animation(.snappy, value: model.label)
                }
            }
            .padding(width / 2)
            .frame(width: g.size.width, height: g.size.height)
        }
        .onAppear { animate(to: model.progress, first: true) }
        .onChange(of: model.progress) { _, value in animate(to: value, first: false) }
    }

    private func animate(to value: Double, first: Bool) {
        if reduceMotion {
            drawn = value
        } else if first {
            withAnimation(.spring(response: 1.0, dampingFraction: 0.82).delay(0.12)) { drawn = value }
        } else {
            withAnimation(.spring(response: 0.6, dampingFraction: 0.78)) { drawn = value }
        }
    }
}

final class RingInstance: ExactNativeInstance {
    private var host: UIViewController!
    private var model: AnyObject!

    init(props: [String: String], events: ExactNativeEvents) {
        super.init(events: events)
        MainActor.assumeIsolated {
            let model = RingModel()
            model.apply(props)
            self.model = model
            self.host = decorative(RingView(model: model))
        }
    }

    override var view: ExactNativeView { MainActor.assumeIsolated { host.view } }
    override var controller: UIViewController? { host }
    override func setProps(_ props: [String: String]) throws {
        MainActor.assumeIsolated { (model as? RingModel)?.apply(props) }
    }
}
