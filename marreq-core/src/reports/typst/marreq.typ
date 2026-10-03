// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq
//
// Layout of Marreq report documents (issue #354). The server passes the
// document model as JSON (`data.json`); every user-provided string arrives as
// a Typst string, never as markup, so it is printed literally.

#let ink = rgb("#0f172a")
#let muted = rgb("#64748b")
#let rule = rgb("#e2e8f0")
#let brand = rgb("#1d4ed8")
#let navy = rgb("#0b1d4d")
#let tones = (
  ok: (fill: rgb("#dcfce7"), text: rgb("#166534"), bar: rgb("#16a34a")),
  warn: (fill: rgb("#fef3c7"), text: rgb("#92400e"), bar: rgb("#d97706")),
  bad: (fill: rgb("#fee2e2"), text: rgb("#991b1b"), bar: rgb("#dc2626")),
  open: (fill: rgb("#e0e7ff"), text: rgb("#3730a3"), bar: rgb("#4f46e5")),
  "none": (fill: rgb("#f1f5f9"), text: rgb("#64748b"), bar: rgb("#cbd5e1")),
)

#let badge(tone, body) = {
  let c = tones.at(tone)
  box(
    fill: c.fill,
    inset: (x: 3pt, y: 1.5pt),
    outset: (y: 0.5pt),
    radius: 2pt,
    text(size: 0.85em, weight: "bold", fill: c.text, body),
  )
}

#let inline(i) = {
  if i.t == "text" { i.text }
  else if i.t == "bold" { strong(i.text) }
  else if i.t == "muted" { text(fill: muted, i.text) }
  else if i.t == "code" { text(font: "DejaVu Sans Mono", size: 0.88em, i.text) }
  else if i.t == "badge" { badge(i.tone, i.text) }
  else if i.t == "line_break" { linebreak() }
}

#let inlines(xs) = {
  for x in xs { inline(x) }
}

#let bar(parts) = {
  let parts = parts.filter(p => p.value > 0)
  let total = parts.map(p => p.value).sum(default: 0)
  if total == 0 {
    box(width: 100%, height: 6pt, fill: tones.at("none").fill, radius: 3pt)
  } else {
    box(width: 100%, height: 6pt, radius: 3pt, clip: true, stack(
      dir: ltr,
      ..parts.map(p => rect(width: p.value / total * 100%, height: 6pt, fill: tones.at(p.tone).bar, stroke: none)),
    ))
  }
}

#let cell-body(c) = if c.t == "bar" { bar(c.parts) } else { inlines(c.inlines) }

#let render-table(t) = {
  let n = t.columns.len()
  let aligns = t.columns.map(c => if c.align == "right" { right } else if c.align == "center" { center } else { left })
  let body = ()
  for r in t.rows {
    if r.kind == "group" {
      body.push(table.cell(
        colspan: n,
        fill: rgb("#eef2ff"),
        text(weight: "bold", fill: rgb("#1e3a8a"), inlines(r.cells.at(0).inlines)),
      ))
    } else {
      for (i, c) in r.cells.enumerate() {
        let content = cell-body(c)
        body.push(table.cell(
          align: aligns.at(i, default: left) + top,
          if r.kind == "total" { strong(content) } else { content },
        ))
      }
    }
  }
  set text(size: if t.compact { 7.5pt } else { 8.5pt })
  table(
    columns: t.columns.map(c => c.width * 1fr),
    stroke: (x, y) => (bottom: 0.5pt + rule),
    fill: (x, y) => if y == 0 { navy },
    inset: (x: 4pt, y: 3.5pt),
    table.header(
      repeat: true,
      ..t.columns.enumerate().map(((i, c)) => table.cell(
        align: aligns.at(i),
        text(weight: "bold", size: 6.8pt, fill: white, upper(c.title)),
      )),
    ),
    ..body,
  )
}

#let kpis(items) = grid(
  columns: items.map(_ => 1fr),
  column-gutter: 6pt,
  ..items.map(k => block(
    width: 100%,
    stroke: (top: 2.5pt + tones.at(k.tone).bar, rest: 0.6pt + rule),
    radius: 3pt,
    inset: 6pt,
  )[
    #text(16pt, weight: "bold", k.value) \
    #text(7.5pt, fill: muted, k.label)
  ]),
)

#let legend(items) = {
  set text(7.5pt, fill: muted)
  for (tone, label) in items {
    box(width: 7pt, height: 7pt, radius: 2pt, fill: tones.at(tone).bar)
    h(3pt)
    label
    h(9pt)
  }
}

#let cover(b) = page(margin: 0pt, header: none, footer: none)[
  #block(
    width: 100%,
    height: 56%,
    fill: gradient.linear(navy, brand, angle: 135deg),
    inset: (x: 22mm, top: 24mm, bottom: 16mm),
  )[
    #set text(fill: white)
    #box(image("logo.png", height: 30pt), baseline: 35%)
    #h(6pt)
    #text(14pt, weight: "bold", "Marreq")
    #v(1fr)
    #text(8.5pt, tracking: 1.5pt, upper(b.kicker))
    #v(4pt)
    #par(leading: 0.35em, text(28pt, weight: "bold", b.title))
    #v(6pt)
    #text(10pt, b.subtitle)
  ]
  #block(inset: (x: 22mm, top: 12mm), width: 100%)[
    #grid(
      columns: (auto, 1fr, auto, 1fr),
      column-gutter: 10pt,
      row-gutter: 7pt,
      ..b.fields.map(f => (text(fill: muted, f.key), strong(f.value))).flatten(),
    )
    #v(12mm)
    #if b.signatories.len() > 0 {
      table(
        columns: (24%, 36%, 25%, 15%),
        stroke: 0.6pt + rgb("#cbd5e1"),
        inset: 8pt,
        table.header(..("", "Name / role", "Signature", "Date").map(h => text(7pt, fill: muted, upper(h)))),
        ..b.signatories.map(s => (strong(s.role), s.name, [], [])).flatten(),
      )
    }
    #if b.note != none {
      v(6mm)
      block(fill: rgb("#fff7ed"), stroke: 0.6pt + rgb("#fdba74"), radius: 4pt, inset: 7pt, text(8pt, b.note))
    }
  ]
]

#let render-heading(b) = {
  let body = if b.number != "" [#b.number#h(0.5em)#b.text] else [#b.text]
  heading(level: b.level, outlined: b.outlined, numbering: none, body)
}

#let render-blocks(blocks) = {
  for b in blocks {
    if b.t == "cover" { cover(b) }
    else if b.t == "toc" { outline(title: b.title, depth: 2) }
    else if b.t == "heading" { render-heading(b) }
    else if b.t == "paragraph" { par(inlines(b.inlines)) }
    else if b.t == "bullets" { list(..b.items.map(inlines)) }
    else if b.t == "table" { render-table(b) }
    else if b.t == "kpis" { kpis(b.items) }
    else if b.t == "legend" { legend(b.items) }
    else if b.t == "page_break" { pagebreak(weak: true) }
    else if b.t == "landscape" { page(flipped: true, render-blocks(b.blocks)) }
  }
}

#let render(doc) = {
  let meta = doc.meta
  let ymd = meta.date.split("-").map(int)
  set document(
    title: meta.title,
    author: "Marreq",
    date: datetime(year: ymd.at(0), month: ymd.at(1), day: ymd.at(2)),
  )
  set text(font: "Inter", size: 9pt, fill: ink, lang: "en")
  set par(leading: 0.62em, spacing: 0.9em)
  let footer-left = (meta.generated_by, meta.classification).filter(s => s != "").join(" · ")
  let header-right = (meta.doc_id, meta.issue_label).filter(s => s != "").join(" · ")
  set page(
    paper: meta.paper,
    margin: (x: 18mm, top: 22mm, bottom: 20mm),
    header: context {
      set text(7.5pt, fill: muted)
      [#box(image("logo.png", height: 9pt), baseline: 15%) #h(3pt) #meta.project · #meta.title #h(1fr) #header-right]
    },
    footer: context {
      set text(7pt, fill: muted)
      [#footer-left #h(1fr) Page #counter(page).display() of #counter(page).final().first()]
    },
    background: if meta.watermark != none and meta.watermark != "" {
      rotate(-30deg, text(90pt, weight: "bold", fill: rgb(29, 78, 216, 14), meta.watermark))
    },
  )
  show heading.where(level: 1): it => block(above: 16pt, below: 7pt, text(13pt, weight: "bold", fill: navy, it.body))
  show heading.where(level: 2): it => block(above: 11pt, below: 5pt, text(10.5pt, weight: "bold", it.body))
  show outline.entry.where(level: 1): set text(weight: "bold")
  render-blocks(doc.blocks)
}
