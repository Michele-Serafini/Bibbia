import SwiftUI
import BibleKit

struct ReadingPlansView: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var plans: [ReadingPlanSummary] = []
    @State private var selected = 0
    @State private var detail: ReadingDetail?
    @State private var date = Date()
    @State private var start = Date()
    @State private var daily = 3
    @State private var scope = false
    @State private var title = ""
    @State private var error = ""
    @State private var deleting = false
    @State private var exporting=false
    private var calendar: Calendar { Calendar.current }
    private var first: Date { calendar.date(from:calendar.dateComponents([.year,.month],from:date))! }
    private var calendarStart:Date { let offset=(calendar.component(.weekday,from:first)+5)%7;return calendar.date(byAdding:.day,value:-offset,to:first)! }
    private func reload() async {
        guard let store=model.store else{return}
        do { plans=try await store.readingPlans();if !plans.contains(where:{$0.id==selected}){selected=plans.first?.id ?? 0}
            detail=selected==0 ? nil:try await store.readingDetail(selected,from:ReadingDates.key(calendarStart),selected:ReadingDates.key(date));error=""
        } catch {self.error=error.localizedDescription}
    }
    private func exportCalendar(save:Bool){let id=selected;exporting=true;Task{do{try await model.importPlanCalendar(id,save:save)}catch{self.error=error.localizedDescription};exporting=false}}
    var body: some View {
        ScrollView { VStack(alignment:.leading,spacing:14) {
            HStack { Text(model.t("Piani di lettura","Reading plans")).font(.title2.bold());Spacer();Button(model.t("Chiudi","Close")){dismiss()} }
            if !plans.isEmpty {
                Picker(model.t("Piano","Plan"),selection:$selected){ForEach(plans){Text($0.title+" · "+$0.edition).tag($0.id)}}
                    .onChange(of:selected){_ in Task{await reload()}}
            }
            if let detail {
                HStack {
                    Text("\(detail.completed) / \(detail.total) · \(Int(Double(detail.completed)*100/Double(max(1,detail.total))))%")
                    Spacer()
                    Button(model.t("Riprendi","Resume")) { Task { do {if let p=try await model.store?.resumeReadingPlan(selected){model.navigate(p);dismiss()}}catch{self.error=error.localizedDescription} } }.accessibilityIdentifier("resumePlan")
                    Button(role:.destructive){deleting=true}label:{Image(systemName:"trash")}
                }
                HStack {
                    Button(model.t("Apri nel calendario di sistema","Open in system calendar")){exportCalendar(save:false)}.accessibilityIdentifier("openPlanCalendar")
                    Button(model.t("Salva calendario .ics…","Save calendar .ics…")){exportCalendar(save:true)}.accessibilityIdentifier("savePlanCalendar")
                    if exporting {ProgressView().controlSize(.small)}
                }.disabled(exporting)
                Text(model.t("Conferma l’importazione nella tua app calendario. È una copia del piano con i progressi attuali; puoi importarla di nuovo quando il piano cambia.","Confirm the import in your calendar app. This is a snapshot of the plan and current progress; import it again when the plan changes.")).font(.caption).foregroundStyle(.secondary)
                ProgressView(value:Double(detail.completed),total:Double(max(1,detail.total)))
                HStack { Button{date=calendar.date(byAdding:.month,value:-1,to:date)!}label:{Image(systemName:"chevron.left")};Spacer();Text(first,format:.dateTime.month(.wide).year());Spacer();Button{date=calendar.date(byAdding:.month,value:1,to:date)!}label:{Image(systemName:"chevron.right")} }
                LazyVGrid(columns:Array(repeating:GridItem(.flexible()),count:7),spacing:5){
                    let weekday=2
                    ForEach(0..<7,id:\.self){i in Text(calendar.shortWeekdaySymbols[(weekday-1+i)%7]).font(.caption).foregroundStyle(.secondary)}
                    ForEach(detail.days,id:\.offset){day in
                        let d=calendar.date(byAdding:.day,value:day.offset,to:calendarStart)!
                        Button {date=d} label:{VStack(spacing:2){Text(d,format:.dateTime.day());Text(day.total==0 ? " ":"\(day.done)/\(day.total)").font(.system(size:9))}.frame(maxWidth:.infinity).padding(4).background(calendar.isDate(d,inSameDayAs:date) ? Color.accentColor.opacity(0.22):day.total>0 && day.done==day.total ? Color.green.opacity(0.16):Color.clear,in:RoundedRectangle(cornerRadius:5))}.buttonStyle(.plain).accessibilityLabel(ReadingDates.key(d)+" · \(day.done)/\(day.total)")
                    }
                }
                Text(date,format:.dateTime.day().month(.wide).year()).font(.headline)
                if detail.chapters.isEmpty {Text(model.t("Nessuna lettura prevista per questo giorno.","No reading scheduled for this day.")).foregroundStyle(.secondary)}
                ForEach(detail.chapters){chapter in
                    HStack {
                        Toggle(isOn:Binding(get:{chapter.complete},set:{value in Task{do{try await model.store?.completeReadingChapter(selected,index:chapter.index,complete:value);await reload()}catch{self.error=error.localizedDescription}}})){Text("\(chapter.name) \(chapter.chapter+1)")}
                        Spacer();Button(model.t("Leggi","Read")){Task{do{try await model.store?.activateReadingPlan(selected);model.navigate(Position(edition:detail.edition,book:chapter.book,chapter:chapter.chapter));dismiss()}catch{self.error=error.localizedDescription}}}
                    }
                }
                Text(model.t("Il punto di lettura si salva automaticamente. Segna i capitoli letti per aggiornare i progressi.","Your reading position is saved automatically. Mark finished chapters to update progress.")).font(.caption).foregroundStyle(.secondary)
                Divider()
            }
            DisclosureGroup(model.t("Nuovo piano","New plan")) {
                VStack(alignment:.leading,spacing:10){
                    TextField(model.t("Titolo del piano","Plan title"),text:$title)
                    Text(model.position.edition).font(.caption).foregroundStyle(.secondary)
                    Toggle(model.t("Solo il libro aperto","Current book only"),isOn:$scope)
                    DatePicker(model.t("Data di inizio","Start date"),selection:$start,displayedComponents:.date)
                    Stepper(model.t("\(daily) capitoli al giorno","\(daily) chapters per day"),value:$daily,in:1...12)
                    Button(model.t("Crea piano","Create plan")){Task{do{selected=try await model.store?.createReadingPlan(title:title.isEmpty ? model.t("La mia lettura","My reading"):title,edition:model.position.edition,start:ReadingDates.key(start),daily:daily,book:scope ? model.position.book:nil) ?? 0;date=start;await reload()}catch{self.error=error.localizedDescription}}}.accessibilityIdentifier("createPlan")
                }.padding(.top,8)
            }
            if !error.isEmpty{Text(error).foregroundStyle(.red).font(.caption)}
        }.padding(24) }.frame(width:540,height:760).task{await reload()}.onChange(of:date){_ in Task{await reload()}}
        .confirmationDialog(model.t("Eliminare il piano e i suoi progressi?","Delete this plan and its progress?"),isPresented:$deleting){Button(model.t("Elimina","Delete"),role:.destructive){Task{do{try await model.store?.deleteReadingPlan(selected);await reload()}catch{self.error=error.localizedDescription}}}}
    }
}
