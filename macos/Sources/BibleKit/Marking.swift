import Foundation

public enum Marking {
    public static let colors = ["yellow", "green", "blue", "pink"]

    public static func validate(text: String, start: Int, length: Int) throws {
        guard start >= 0, length > 0, start <= text.utf16.count,
              length <= text.utf16.count - start else {
            throw BibleError.invalid("Selezione non valida / Invalid selection")
        }
        let units = text.utf16
        func boundary(_ offset: Int) -> Bool {
            if offset == units.count { return true }
            let unit = units[units.index(units.startIndex, offsetBy: offset)]
            return !(0xdc00...0xdfff).contains(unit)
        }
        guard boundary(start), boundary(start + length) else {
            throw BibleError.invalid("Selezione Unicode non valida / Invalid Unicode selection")
        }
    }

    public static func apply(text: String, annotation: Annotation,
                             start: Int, length: Int, action: String) throws -> Annotation {
        try validate(text: text, start: start, length: length)
        guard colors.contains(action) || ["underline", "clear"].contains(action) else {
            throw BibleError.invalid("Azione non valida / Invalid action")
        }
        var boundaries = Set([0, text.utf16.count, start, start + length])
        for mark in annotation.ranges {
            try validate(text: text, start: mark.start, length: mark.length)
            boundaries.insert(mark.start); boundaries.insert(mark.start + mark.length)
        }
        let sorted = boundaries.sorted()
        var ranges: [TextMark] = []
        for (a, b) in zip(sorted, sorted.dropFirst()) {
            var style = TextMark(start: a, length: b - a, highlight: annotation.highlight,
                                 underline: annotation.underline)
            for mark in annotation.ranges where a >= mark.start && b <= mark.start + mark.length {
                if let color = mark.highlight { style.highlight = color }
                style.underline = style.underline || mark.underline
            }
            if a >= start && b <= start + length {
                if action == "clear" { style.highlight = nil; style.underline = false }
                else if action == "underline" { style.underline = true }
                else { style.highlight = action }
            }
            guard style.highlight != nil || style.underline else { continue }
            if let last = ranges.last, last.start + last.length == a,
               last.highlight == style.highlight, last.underline == style.underline {
                ranges[ranges.count - 1].length += style.length
            } else { ranges.append(style) }
        }
        guard ranges.count <= 512 else { throw BibleError.invalid("Troppe annotazioni / Too many marks") }
        var result = annotation
        result.highlight = nil; result.underline = false; result.ranges = ranges
        return result
    }
}
