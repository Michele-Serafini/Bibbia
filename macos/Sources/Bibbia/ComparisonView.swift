import SwiftUI
import BibleKit

struct ComparisonView: View {
    @ObservedObject var model: AppModel
    private var notice: String {
        switch model.comparison?.status {
        case "missingBook": model.t("Questo libro non è presente nella seconda traduzione.","This book is absent from the second translation.")
        case "differentNumbering": model.t("Numerazione diversa: corrispondenza non verificata per questo capitolo.","Different numbering: no verified alignment for this chapter.")
        case "partial": model.t("Alcuni riferimenti non hanno una corrispondenza verificata. I riferimenti originali sono conservati.","Some references have no verified counterpart. Original references are preserved.")
        default: model.t("Versetti allineati · scorrimento sincronizzato · ↗ per leggere e annotare.","Aligned verses · synchronized scrolling · ↗ to read and annotate.")
        }
    }
    var body: some View {
        VStack(spacing: 12) {
            HStack(spacing: 24) {
                Text(model.position.edition).font(.headline).frame(maxWidth: .infinity,alignment: .leading)
                Picker(model.t("Seconda traduzione","Second translation"),selection: $model.preferences.comparisonEdition) {
                    ForEach(model.editions.filter { $0.id != model.position.edition }) { Text($0.id).tag($0.id) }
                }.labelsHidden().frame(maxWidth: .infinity).accessibilityIdentifier("comparisonEdition")
                    .onChange(of: model.preferences.comparisonEdition) { _ in model.preferencesChanged(); model.refreshComparison() }
            }.padding(.horizontal,24)
            Text(notice).font(.caption).foregroundStyle(.secondary).frame(maxWidth: .infinity,alignment: .leading).padding(.horizontal,24)
            if model.comparisonLoading { ProgressView().frame(maxWidth: .infinity,maxHeight: .infinity) }
            else if let comparison = model.comparison {
                ComparisonDocuments(model: model, comparison: comparison)
            }
        }.padding(.top,8)
    }
}
