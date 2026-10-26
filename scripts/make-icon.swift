// Generates the placeholder app icon: swift scripts/make-icon.swift assets/icon/alpha.icns
import Cocoa

let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "assets/icon/alpha.icns"
let sizes = [16, 32, 64, 128, 256, 512, 1024]
let tmp = NSTemporaryDirectory() + "alpha.iconset"
try? FileManager.default.removeItem(atPath: tmp)
try! FileManager.default.createDirectory(atPath: tmp, withIntermediateDirectories: true)

func render(_ px: Int) -> Data {
    let s = CGFloat(px)
    let img = NSImage(size: NSSize(width: s, height: s))
    img.lockFocus()
    let rect = NSRect(x: s * 0.06, y: s * 0.06, width: s * 0.88, height: s * 0.88)
    NSColor(calibratedRed: 0.11, green: 0.105, blue: 0.10, alpha: 1).setFill()
    NSBezierPath(roundedRect: rect, xRadius: s * 0.2, yRadius: s * 0.2).fill()
    let font = NSFont(name: "Georgia", size: s * 0.62) ?? NSFont.systemFont(ofSize: s * 0.62)
    let attrs: [NSAttributedString.Key: Any] = [
        .font: font,
        .foregroundColor: NSColor(calibratedRed: 0.86, green: 0.65, blue: 0.50, alpha: 1),
    ]
    let str = NSAttributedString(string: "a", attributes: attrs)
    let sz = str.size()
    str.draw(at: NSPoint(x: (s - sz.width) / 2, y: (s - sz.height) / 2 - s * 0.02))
    img.unlockFocus()
    let rep = NSBitmapImageRep(data: img.tiffRepresentation!)!
    return rep.representation(using: .png, properties: [:])!
}

for px in sizes {
    let data = render(px)
    if px <= 512 { try! data.write(to: URL(fileURLWithPath: "\(tmp)/icon_\(px)x\(px).png")) }
    if px >= 32 && px / 2 <= 512 { try! data.write(to: URL(fileURLWithPath: "\(tmp)/icon_\(px / 2)x\(px / 2)@2x.png")) }
}
let p = Process()
p.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
p.arguments = ["-c", "icns", tmp, "-o", out]
try! p.run(); p.waitUntilExit()
exit(p.terminationStatus)
