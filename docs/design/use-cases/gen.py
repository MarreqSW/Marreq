"""Static UX prototype for issue #388: interactive use case editor and explorer."""
import os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "mockups")

FONTS = (
    '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Public+Sans:wght@400;600;700;800'
    '&family=Google+Sans+Text:wght@400;500;700&family=Inter:wght@300;400;500;600;700'
    '&family=JetBrains+Mono:wght@400;500;600&display=swap">'
)


def icon(name, cls="text-base"):
    return f'<span class="material-symbols-outlined {cls}" aria-hidden="true">{name}</span>'


# ── Shell ──────────────────────────────────────────────────────────────
NAV = [
    ("dashboard", "Dashboard"),
    ("list_alt", "Requirements"),
    ("verified", "Verifications"),
    ("account_tree", "Use cases"),
    ("hub", "Traceability"),
    ("history_edu", "Baselines"),
    ("description", "Reports & exports"),
]


def sidebar(active):
    items = []
    for ic, label in NAV:
        on = label == active
        new = (' <span class="ml-auto rounded-sm bg-amber-200 text-amber-900 px-1 py-px text-[9px] tracking-wider">NEW</span>'
               if label == "Use cases" else "")
        cls = ("bg-stitch-elevated text-stitch-accent border-l-4 border-stitch-accent pl-4"
               if on else "text-stitch-fg pl-5")
        items.append(f'<div class="flex items-center gap-3 py-3.5 pr-4 rounded-r-md text-[12px] font-bold uppercase tracking-[0.12em] {cls}">'
                     f'{icon(ic, "text-lg")}<span>{label}</span>{new}</div>')
    return f"""
<aside class="w-60 shrink-0 bg-stitch-surface border-r border-stitch-border flex flex-col">
  <div class="flex items-center gap-3 px-6 h-[70px]">
    <div class="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">{icon('architecture', 'text-white text-sm')}</div>
    <div><p class="text-stitch-accent text-xs uppercase tracking-wider font-bold leading-none">Marreq</p>
    <p class="text-[10px] text-stitch-muted font-mono leading-none mt-1">v0.1.18</p></div>
  </div>
  <nav class="mt-4 pr-6 space-y-1">{''.join(items)}</nav>
  <div class="mt-auto border-t border-stitch-border px-5 py-4 space-y-4 text-[12px] font-bold uppercase tracking-[0.12em] text-stitch-fg">
    <div class="flex items-center gap-3">{icon('settings', 'text-lg')} Project settings</div>
    <div class="flex items-center gap-3">{icon('help', 'text-lg')} Help</div>
  </div>
</aside>"""


def topbar(primary="Create use case"):
    return f"""
<header class="h-[70px] shrink-0 bg-stitch-surface border-b border-stitch-border shadow-sm px-6 flex items-center gap-4">
  <span class="text-xl font-bold text-stitch-fg font-headline">Space Project</span>
  <span class="flex items-center gap-1.5 border border-stitch-border rounded-md px-2.5 py-1.5 text-xs bg-stitch-elevated">{icon('swap_horiz', 'text-sm')}Projects{icon('expand_more', 'text-sm')}</span>
  <div class="flex-1 max-w-md flex items-center gap-2 border border-stitch-border rounded-md bg-stitch-elevated px-3 py-2 text-sm text-stitch-muted">{icon('search', 'text-sm')}Global search…</div>
  <div class="ml-auto flex items-center gap-4 text-stitch-muted">
    {icon('notifications', 'text-xl')}
    <span class="inline-flex items-center gap-2 rounded-md bg-linear-to-br from-primary to-primary-container text-white px-4 py-2.5 text-sm font-semibold shadow-lg">{icon('add', 'text-sm')}{primary}</span>
    <span class="w-9 h-9 rounded-full border-2 border-stitch-border bg-stitch-elevated text-stitch-accent text-xs font-bold flex items-center justify-center">AJ</span>
  </div>
</header>"""


def page(title, caption, body, active="Use cases", primary="Create use case", overlay=""):
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>{title}</title>
{FONTS}<link rel="stylesheet" href="./style.css"></head>
<body class="font-sans text-stitch">
<div class="h-screen flex bg-stitch-canvas overflow-hidden">
{sidebar(active)}
<div class="flex-1 min-w-0 flex flex-col">
{topbar(primary)}
<main class="flex-1 min-h-0 overflow-hidden px-8 py-5 relative">{body}</main>
<footer class="shrink-0 border-t border-stitch-border bg-stitch-surface px-6 py-1.5 flex justify-between font-mono text-[10px] text-stitch-muted">
  <span>{caption}</span><span>Mockup for #388 · illustrative data</span>
</footer>
</div>{overlay}</div></body></html>"""


# ── Small pieces ───────────────────────────────────────────────────────
def chip(text, cls="bg-stitch-elevated text-stitch-fg"):
    return f'<span class="rounded-sm px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider {cls}">{text}</span>'


def code(text, cls=""):
    return f'<span class="font-mono text-[11px] font-semibold text-stitch-accent whitespace-nowrap {cls}">{text}</span>'


def link_chip(text, suspect=False):
    if suspect:
        return (f'<span class="inline-flex items-center gap-1 rounded-full border border-amber-400 bg-amber-50 px-2 py-0.5 '
                f'font-mono text-[10px] font-semibold text-amber-800">{icon("link_off", "text-[12px]")}{text}</span>')
    return (f'<span class="inline-flex items-center gap-1 rounded-full border border-stitch-accent/30 bg-stitch-accent/5 px-2 py-0.5 '
            f'font-mono text-[10px] font-semibold text-stitch-accent">{icon("link", "text-[12px]")}{text}</span>')


def btn(label, ic=None, cls=""):
    i = icon(ic, "text-sm text-stitch-muted") if ic else ""
    return (f'<span class="inline-flex items-center gap-1.5 rounded-md border border-stitch-border bg-stitch-surface '
            f'text-stitch-fg px-3 py-1.5 text-xs font-semibold whitespace-nowrap {cls}">{i}{label}</span>')


def btn_primary(label, ic=None):
    i = icon(ic, "text-sm") if ic else ""
    return (f'<span class="inline-flex items-center gap-1.5 rounded-md bg-linear-to-br from-primary to-primary-container '
            f'text-white px-3 py-1.5 text-xs font-semibold shadow-md whitespace-nowrap">{i}{label}</span>')


def section(title, ic, inner, extra=""):
    return f"""
<section class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm">
  <div class="flex items-center gap-2 px-4 py-2.5 border-b border-stitch-border bg-stitch-elevated/50">
    {icon(ic, 'text-lg text-stitch-accent')}<h3 class="text-sm font-bold text-stitch-accent font-headline">{title}</h3>{extra}
  </div>
  <div class="px-4 py-2">{inner}</div>
</section>"""


def bullet(text, extra=""):
    return f'<div class="group flex items-start gap-2 py-1.5 text-sm text-stitch-fg">{icon("drag_indicator", "text-base text-transparent")}<span class="mt-[7px] w-1.5 h-1.5 rounded-full bg-stitch-muted shrink-0"></span><span class="flex-1">{text}</span>{extra}</div>'


def step(num, text, links="", state="", lead="", tail="", numcls="bg-stitch-accent text-white"):
    """A numbered step block. state: '', 'hover', 'edit', 'origin', 'ghost', 'dim'."""
    base = "relative flex items-start gap-2 rounded-md px-1 py-1.5"
    handle = icon("drag_indicator", "text-base text-transparent mt-0.5")
    body = f'<span class="flex-1 text-sm text-stitch-fg leading-6">{text}</span>'
    if state == "hover":
        base += " bg-stitch-elevated/60"
        handle = icon("drag_indicator", "text-base text-stitch-muted mt-0.5 cursor-grab")
    elif state == "edit":
        base += " bg-stitch-surface ring-2 ring-stitch-accent shadow-md"
        handle = icon("drag_indicator", "text-base text-stitch-muted mt-0.5")
        body = (f'<span class="flex-1 text-sm text-stitch-fg leading-6">{text}'
                f'<span class="inline-block w-px h-4 bg-stitch-accent align-middle animate-pulse ml-px"></span></span>')
    elif state == "origin":
        base += " bg-amber-50 ring-1 ring-amber-400"
    elif state == "ghost":
        base += " opacity-35 border border-dashed border-stitch-border"
    elif state == "dim":
        base += " opacity-60"
    return f"""
<div class="{base}">{lead}{handle}
  <span class="mt-0.5 w-6 h-6 shrink-0 rounded-full {numcls} text-[11px] font-bold flex items-center justify-center">{num}</span>
  {body}<span class="flex flex-wrap justify-end items-center gap-1 mt-0.5 shrink-0">{links}</span>{tail}
</div>"""


def branch_card(kind, label, title, origin, steps, open_=False, highlight=False):
    alt = kind == "alt"
    colour = "border-l-sky-600" if alt else "border-l-rose-600"
    tag = chip("Alternative", "bg-sky-100 text-sky-800") if alt else chip("Exception", "bg-rose-100 text-rose-800")
    ring = " ring-2 ring-amber-400" if highlight else ""
    head = f"""
  <div class="flex items-center gap-2 px-3 py-2">
    {icon('expand_more' if open_ else 'chevron_right', 'text-base text-stitch-muted')}
    <span class="font-mono text-xs font-bold text-stitch-fg">{label}</span>{tag}
    <span class="text-sm font-semibold text-stitch-fg whitespace-nowrap">{title}</span>
    <span class="ml-auto flex items-center gap-1 text-xs text-stitch-muted whitespace-nowrap">{icon('subdirectory_arrow_right', 'text-sm')}branches from <strong class="text-stitch-fg">step {origin}</strong></span>
  </div>"""
    body = ""
    if open_:
        rows = "".join(
            f'<div class="flex items-start gap-2 py-1"><span class="font-mono text-[11px] font-bold text-stitch-muted w-10 shrink-0 pt-0.5">{n}</span><span class="text-sm text-stitch-fg">{t}</span></div>'
            for n, t in steps)
        body = f'<div class="px-3 pb-2 pl-9 border-t border-stitch-border/60 pt-1.5">{rows}</div>'
    return f'<div class="rounded-md border border-stitch-border border-l-4 {colour} bg-stitch-surface{ring}">{head}{body}</div>'


def uc_header(right=""):
    return f"""
<div class="flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.14em] text-stitch-muted mb-2">
  Use cases {icon('chevron_right', 'text-sm')} <span class="text-stitch-accent">UC-001</span>
</div>
<div class="flex items-start justify-between mb-3">
  <div>
    <div class="flex items-center gap-2 mb-1">{code('UC-001', 'text-xs')}{chip('Approved', 'bg-emerald-100 text-emerald-800')}{chip('v2', 'bg-stitch-elevated text-stitch-fg')}
      <span class="text-xs text-stitch-muted">Primary actor <strong class="text-stitch-fg">Project member</strong> · Owner <strong class="text-stitch-fg">Alice Johnson</strong></span></div>
    <h1 class="text-2xl font-bold text-stitch-fg font-headline">Export requirements</h1>
  </div>
  <div class="flex gap-2">{right}</div>
</div>"""


PRE = ["User is authenticated.", "User has access to the project."]
MAIN = [
    ("User opens the requirements page.", ""),
    ("System displays the available requirements.", link_chip("REQ-UI-003")),
    ("User selects requirements and requests an export.", link_chip("REQ-EXP-004")),
    ("System generates the file in the chosen format.", link_chip("REQ-EXP-007") + link_chip("TEST-EXP-002")),
]
POST = ["The export file is available to the user.", "The export is recorded in the audit log."]


def main_steps(states=None, links=True):
    states = states or {}
    return "".join(step(i + 1, t, l if links else "", states.get(i + 1, "")) for i, (t, l) in enumerate(MAIN))


# ── 1 · List ───────────────────────────────────────────────────────────
def s1_list():
    rows = [
        ("UC-001", "Export requirements", "Project member", 4, "1 · 1", 3, ("Approved", "bg-emerald-100 text-emerald-800"), "2 h ago"),
        ("UC-002", "Import a ReqIF package", "Project admin", 6, "2 · 1", 5, ("Reviewed", "bg-sky-100 text-sky-800"), "yesterday"),
        ("UC-003", "Approve a requirement version", "Reviewer", 5, "1 · 2", 4, ("Approved", "bg-emerald-100 text-emerald-800"), "3 d ago"),
        ("UC-004", "Downlink telemetry pass", "Ground operator", 9, "3 · 2", 7, ("Draft", "bg-slate-200 text-slate-700"), "20 min ago"),
        ("UC-005", "Safe mode entry", "On-board software", 7, "1 · 3", 6, ("Draft", "bg-slate-200 text-slate-700"), "1 h ago"),
        ("UC-006", "Payload calibration", "Payload engineer", 8, "2 · 1", 2, ("Reviewed", "bg-sky-100 text-sky-800"), "last week"),
        ("UC-007", "Baseline the project", "Project admin", 3, "0 · 1", 2, ("Approved", "bg-emerald-100 text-emerald-800"), "2 weeks ago"),
    ]
    trs = "".join(f"""
<tr class="border-t border-stitch-border {'bg-stitch-elevated/40' if i == 3 else ''}">
  <td class="px-4 py-3">{code(c)}</td>
  <td class="px-4 py-3 font-semibold text-stitch-fg">{t}</td>
  <td class="px-4 py-3 text-stitch-muted">{a}</td>
  <td class="px-4 py-3 text-center">{s}</td>
  <td class="px-4 py-3 text-center text-stitch-muted">{b}</td>
  <td class="px-4 py-3 text-center">{icon('link', 'text-sm text-stitch-muted align-middle')} {r}</td>
  <td class="px-4 py-3">{chip(st[0], st[1])}</td>
  <td class="px-4 py-3 text-stitch-muted text-xs">{u}</td>
</tr>""" for i, (c, t, a, s, b, r, st, u) in enumerate(rows))
    body = f"""
<div class="flex items-end justify-between mb-4">
  <div><p class="text-[11px] font-bold uppercase tracking-[0.14em] text-stitch-muted">Space Project</p>
  <h1 class="text-2xl font-bold text-stitch-fg font-headline">Use cases</h1>
  <p class="text-sm text-stitch-muted">How the system behaves, step by step. Linked to the requirements and verifications that cover it.</p></div>
</div>
<div class="flex gap-2 mb-3">
  <div class="flex-1 max-w-sm flex items-center gap-2 border border-stitch-border rounded-md bg-stitch-surface px-3 py-1.5 text-sm text-stitch-muted">{icon('search', 'text-sm')}Filter by code, title or actor…</div>
  {btn('Actor: All', 'person')}{btn('State: All', 'approval')}{btn('Has suspect links', 'link_off')}
</div>
<div class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm overflow-hidden">
<table class="w-full text-sm">
  <thead class="bg-stitch-elevated text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">
    <tr><th class="px-4 py-2.5 text-left">Code</th><th class="px-4 py-2.5 text-left">Title</th><th class="px-4 py-2.5 text-left">Primary actor</th>
    <th class="px-4 py-2.5">Steps</th><th class="px-4 py-2.5">Alt · Exc</th><th class="px-4 py-2.5">Linked</th><th class="px-4 py-2.5 text-left">State</th><th class="px-4 py-2.5 text-left">Updated</th></tr>
  </thead><tbody>{trs}</tbody>
</table>
</div>
<p class="mt-3 text-xs text-stitch-muted">7 use cases · <span class="text-amber-700 font-semibold">{icon('link_off', 'text-xs align-middle')} 1 with suspect links</span></p>"""
    return page("1 · Use case list", "1 · Use case list", body)


# ── 2 · Document view ──────────────────────────────────────────────────
def related_panel(extra_top=""):
    reqs = [("REQ-UI-003", "Requirements list shows code, title, state", "step 2"),
            ("REQ-EXP-004", "Export a selection of requirements", "step 3"),
            ("REQ-EXP-007", "Export formats: XLSX, CSV, ReqIF, PDF", "step 4")]
    req_rows = "".join(f"""
<div class="py-2 border-t border-stitch-border first:border-t-0">
  <div class="flex items-center gap-2">{code(c)}<span class="ml-auto text-[10px] text-stitch-muted">{s}</span></div>
  <p class="text-xs text-stitch-fg">{t}</p></div>""" for c, t, s in reqs)
    return f"""
<aside class="w-72 shrink-0 space-y-3">
  {extra_top}
  <div class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm">
    <div class="px-4 py-2.5 border-b border-stitch-border flex items-center gap-2">{icon('hub', 'text-lg text-stitch-accent')}<h3 class="text-sm font-bold text-stitch-accent font-headline">Related artifacts</h3></div>
    <div class="px-4 py-2">
      <p class="text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted mt-1">Requirements (3)</p>{req_rows}
      <p class="text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted mt-3">Verifications (1)</p>
      <div class="py-2 flex items-center gap-2">{code('TEST-EXP-002')}<span class="text-xs text-stitch-fg">Export file round-trip</span>{chip('Passed', 'bg-emerald-600 text-white ml-auto')}</div>
      <p class="text-[10px] text-stitch-muted border-t border-stitch-border pt-2 mt-1">Linking a verification here does not mark the requirements as verified.</p>
    </div>
  </div>
  <div class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm px-4 py-3">
    <p class="text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted mb-1">Views</p>
    <div class="flex gap-1">{btn('Document', 'article', 'bg-stitch-elevated text-stitch-accent')}{btn('Flow', 'schema')}</div>
  </div>
</aside>"""


def s2_document():
    doc = f"""
<div class="flex-1 min-w-0 space-y-3">
  {section('Preconditions', 'login', ''.join(bullet(t) for t in PRE))}
  {section('Main scenario', 'format_list_numbered', main_steps(), '<span class="ml-auto text-xs text-stitch-muted">4 steps</span>')}
  <div class="space-y-2">
    {branch_card('alt', 'A3', 'Nothing selected', 3, [])}
    {branch_card('exc', 'E4', 'Export fails', 4, [])}
  </div>
  {section('Postconditions', 'task_alt', ''.join(bullet(t) for t in POST))}
</div>"""
    body = uc_header(btn('Compare versions', 'compare') + btn('Comments (3)', 'forum') + btn_primary('Edit', 'edit')) + \
        f'<div class="flex gap-5">{doc}{related_panel()}</div>'
    return page("2 · Document view", "2 · Reading a use case: the document view", body)


# ── 3 · Inline editing ─────────────────────────────────────────────────
def s3_edit():
    menu = f"""
<div class="absolute right-2 top-9 z-20 w-80 rounded-lg border border-stitch-border bg-stitch-surface shadow-stitch py-1 text-sm">
  {''.join(f'<div class="flex items-center gap-2.5 px-3 py-1.5 {cls}">{icon(i, "text-base text-stitch-muted")}{t}<span class="ml-auto font-mono text-[10px] text-stitch-muted">{k}</span></div>'
           for i, t, k, cls in [
               ('alt_route', 'Add alternative from this step', 'A', 'bg-stitch-elevated'),
               ('report', 'Add exception from this step', 'E', ''),
               ('link', 'Link requirement or verification', 'L', ''),
               ('add_comment', 'Comment on this step', 'C', ''),
               ('content_copy', 'Duplicate', '⌘D', ''),
           ])}
  <div class="border-t border-stitch-border mt-1 pt-1 flex items-center gap-2.5 px-3 py-1.5 text-stitch-danger">{icon('delete', 'text-base')}Delete step<span class="ml-auto font-mono text-[10px]">⌫</span></div>
</div>"""
    tools = (f'<span class="flex items-center gap-0.5 ml-1 text-stitch-muted">'
             f'<span class="p-0.5 rounded-sm hover:bg-stitch-elevated">{icon("add", "text-base")}</span>'
             f'<span class="p-0.5 rounded-sm bg-stitch-elevated text-stitch-fg">{icon("more_horiz", "text-base")}</span></span>')
    steps = (step(1, MAIN[0][0]) + step(2, MAIN[1][0], MAIN[1][1])
             + f'<div class="relative">{step(3, "User selects requirements and requests an export in a chosen format", MAIN[2][1], "edit", tail=tools)}{menu}</div>'
             + step(4, MAIN[3][0], MAIN[3][1])
             + f'<div class="flex items-center gap-2 pl-9 py-1.5 text-sm text-stitch-muted">{icon("add", "text-base")}Add step <span class="text-xs">· or press Enter at the end of a step</span></div>')
    draft = f"""
<div class="flex items-center gap-3 rounded-lg border border-amber-300 bg-amber-50 px-4 py-2 mb-3">
  {icon('edit_note', 'text-lg text-amber-700')}<span class="text-sm text-amber-900"><strong>Draft</strong> · unsaved changes · based on v2</span>
  <span class="ml-auto flex items-center gap-2">{btn('', 'undo')}{btn('', 'redo')}{btn('Save draft', 'save')}{btn_primary('Submit', 'send')}</span>
</div>"""
    doc = f"""
<div class="flex-1 min-w-0 space-y-3">
  {draft}
  {section('Preconditions', 'login', ''.join(bullet(t) for t in PRE) + f'<div class="flex items-center gap-2 pl-6 py-1 text-xs text-stitch-muted">{icon("add", "text-sm")}Add precondition</div>')}
  {section('Main scenario', 'format_list_numbered', steps)}
</div>"""
    tip = f"""
<div class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm px-4 py-3 text-xs text-stitch-muted space-y-1.5">
  <p class="text-[10px] font-bold uppercase tracking-[0.12em]">Keyboard</p>
  <p><span class="font-mono text-stitch-fg">Enter</span> new step below · <span class="font-mono text-stitch-fg">Tab</span> indent into a branch</p>
  <p><span class="font-mono text-stitch-fg">Alt ↑ / ↓</span> move the step · <span class="font-mono text-stitch-fg">⌘Z</span> undo</p>
  <p><span class="font-mono text-stitch-fg">/</span> commands (link, comment, alternative…)</p>
</div>"""
    body = uc_header(btn('Cancel') ) + f'<div class="flex gap-5">{doc}<aside class="w-72 shrink-0 space-y-3">{tip}</aside></div>'
    return page("3 · Inline editing", "3 · Editing in place: block toolbar and step menu", body)


# ── 4 · Reordering ─────────────────────────────────────────────────────
def s4_reorder():
    drop = '<div class="relative h-0"><div class="absolute left-8 right-2 -top-px h-0.5 bg-stitch-accent rounded-full"></div><span class="absolute left-6 -top-1.5 w-3 h-3 rounded-full border-2 border-stitch-accent bg-stitch-surface"></span></div>'
    dragged = f"""
<div class="relative z-10 -mt-1 mb-1 ml-10 mr-6 rotate-[-1deg] rounded-md bg-stitch-surface shadow-stitch ring-2 ring-stitch-accent px-1 py-1.5 flex items-start gap-2">
  {icon('drag_indicator', 'text-base text-stitch-accent mt-0.5 cursor-grabbing')}
  <span class="mt-0.5 w-6 h-6 shrink-0 rounded-full bg-stitch-accent text-white text-[11px] font-bold flex items-center justify-center">3</span>
  <span class="flex-1 text-sm text-stitch-fg leading-6">User selects requirements and requests an export.</span>
  <span class="flex items-center gap-1 mt-0.5">{link_chip('REQ-EXP-004')}{chip('A3 follows', 'bg-sky-100 text-sky-800')}</span>
</div>"""
    before = (step(1, MAIN[0][0]) + drop + dragged + step(2, MAIN[1][0], MAIN[1][1])
              + step(3, MAIN[2][0], MAIN[2][1], "ghost") + step(4, MAIN[3][0], MAIN[3][1]))
    after_rows = [
        (1, MAIN[0][0], ""),
        (2, "User selects requirements and requests an export.", "moved"),
        (3, "System displays the available requirements.", "moved"),
        (4, MAIN[3][0], ""),
    ]
    after = "".join(
        step(n, t, chip('was 3', 'bg-violet-100 text-violet-800') if n == 2 else (chip('was 2', 'bg-violet-100 text-violet-800') if m else ""),
             "origin" if n == 2 else "")
        for n, t, m in after_rows) + f'<div class="mt-2">{branch_card("alt", "A2", "Nothing selected", 2, [], highlight=True)}</div>'
    body = f"""
{uc_header(btn('Cancel'))}
<div class="grid grid-cols-2 gap-5">
  <div>
    <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-stitch-muted mb-2">While dragging step 3 above step 2</p>
    {section('Main scenario', 'format_list_numbered', before)}
    <p class="mt-2 text-xs text-stitch-muted flex items-center gap-1.5">{icon('keyboard', 'text-sm')}Keyboard: focus the step, then <span class="font-mono text-stitch-fg">Alt ↑</span>. Screen readers hear “Step 3 moved to position 2”.</p>
  </div>
  <div>
    <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-stitch-muted mb-2">After the drop</p>
    {section('Main scenario', 'format_list_numbered', after)}
    <div class="mt-3 rounded-lg bg-stitch-fg text-white px-4 py-2.5 shadow-stitch flex items-center gap-3 text-sm">
      {icon('check_circle', 'text-lg text-emerald-300')}
      <span>Steps renumbered. <strong>Alternative “Nothing selected”</strong> followed its step: now <strong>A2</strong>, branching from step 2.</span>
      <span class="ml-auto font-semibold text-sky-300">Undo</span>
    </div>
    <p class="mt-2 text-xs text-stitch-muted">Branches, links and comments point at the step itself, not at its number, so nothing needs repairing.</p>
  </div>
</div>"""
    return page("4 · Reordering", "4 · Reordering steps: references follow the step, numbers follow the order", body)


# ── 5 · Branches ───────────────────────────────────────────────────────
def s5_branches():
    states = {3: "origin"}
    main = "".join(step(i + 1, t, "", states.get(i + 1, "")) for i, (t, l) in enumerate(MAIN))
    a3 = branch_card('alt', 'A3', 'Nothing selected', 3, [
        ("A3.1", "System asks the user to select at least one requirement."),
        ("A3.2", "User selects requirements."),
        ("A3.3", "Use case continues at <strong>step 3</strong>."),
    ], open_=True, highlight=True)
    e4 = branch_card('exc', 'E4', 'Export fails', 4, [
        ("E4.1", "System reports the error and keeps the selection."),
        ("E4.2", "System records the failure in the audit log."),
        ("E4.3", "Use case ends <strong>without</strong> an export file."),
    ], open_=True)
    note = f"""
<div class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm px-4 py-3 text-xs space-y-2">
  <p class="text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">Selected: A3</p>
  <p class="text-stitch-fg flex items-start gap-1.5">{icon('subdirectory_arrow_right', 'text-sm text-amber-600')}<span>Branches from <strong>step 3</strong>, highlighted in the main scenario.</span></p>
  <p class="text-stitch-accent font-semibold flex items-center gap-1.5">{icon('north_west', 'text-sm')}Jump to origin</p>
  <p class="text-stitch-accent font-semibold flex items-center gap-1.5">{icon('undo', 'text-sm')}Rejoins at step 3</p>
  <p class="text-stitch-muted border-t border-stitch-border pt-2">Ends with: <em>continues at step N</em> or <em>use case ends</em>. Both are explicit, never implied by position.</p>
</div>"""
    body = uc_header(btn('Collapse all branches', 'unfold_less') + btn_primary('Edit', 'edit')) + f"""
<div class="flex gap-5">
  <div class="flex-1 min-w-0 grid grid-cols-2 gap-4 items-start">
    {section('Main scenario', 'format_list_numbered', main)}
    <div class="space-y-3">
      <p class="text-[11px] font-bold uppercase tracking-[0.14em] text-stitch-muted">Alternatives &amp; exceptions (2)</p>
      {a3}{e4}
    </div>
  </div>
  <aside class="w-64 shrink-0">{note}</aside>
</div>"""
    return page("5 · Branches", "5 · Alternatives and exceptions next to the main flow", body)


# ── 6 · Delete a referenced step ───────────────────────────────────────
def s6_delete():
    behind = uc_header(btn('Cancel')) + f"""
<div class="flex gap-5 opacity-60">
  <div class="flex-1 min-w-0 space-y-3">{section('Main scenario', 'format_list_numbered', main_steps({3: 'edit'}))}
  {branch_card('alt', 'A3', 'Nothing selected', 3, [])}</div>{related_panel()}
</div>"""
    dialog = f"""
<div class="fixed inset-0 z-50 bg-black/35 flex items-center justify-center">
  <div class="w-[540px] rounded-xl bg-stitch-surface shadow-stitch border border-stitch-border">
    <div class="px-6 pt-5 pb-3 flex items-start gap-3">
      <span class="w-10 h-10 rounded-full bg-red-100 text-stitch-danger flex items-center justify-center shrink-0">{icon('delete', 'text-xl')}</span>
      <div><h2 class="text-lg font-bold text-stitch-fg font-headline">Delete step 3?</h2>
      <p class="text-sm text-stitch-muted mt-0.5">“User selects requirements and requests an export.” is referenced:</p></div>
    </div>
    <ul class="mx-6 rounded-lg border border-stitch-border divide-y divide-stitch-border text-sm">
      <li class="px-3 py-2 flex items-center gap-2">{icon('alt_route', 'text-base text-sky-700')}Origin of <strong>A3 · Nothing selected</strong></li>
      <li class="px-3 py-2 flex items-center gap-2">{icon('link', 'text-base text-stitch-accent')}Linked to {code('REQ-EXP-004')}</li>
      <li class="px-3 py-2 flex items-center gap-2">{icon('forum', 'text-base text-stitch-muted')}1 open review comment</li>
    </ul>
    <p class="px-6 pt-4 pb-2 text-xs font-bold uppercase tracking-[0.12em] text-stitch-muted">What should happen to A3?</p>
    <div class="px-6 space-y-2 text-sm">
      <label class="flex items-start gap-2 rounded-lg border-2 border-stitch-accent bg-stitch-accent/5 px-3 py-2"><span class="mt-0.5 w-4 h-4 rounded-full border-4 border-stitch-accent"></span><span><strong>Branch from step 2 instead</strong><br><span class="text-xs text-stitch-muted">“System displays the available requirements.”</span></span></label>
      <label class="flex items-start gap-2 rounded-lg border border-stitch-border px-3 py-2"><span class="mt-0.5 w-4 h-4 rounded-full border-2 border-stitch-border"></span><span><strong>Delete A3 too</strong><br><span class="text-xs text-stitch-muted">Its 3 steps are removed with it.</span></span></label>
    </div>
    <p class="px-6 pt-3 text-xs text-stitch-muted">The link to REQ-EXP-004 is removed; the comment stays in the history of v2.</p>
    <div class="px-6 py-4 mt-2 flex justify-end gap-2 border-t border-stitch-border">{btn('Cancel')}<span class="inline-flex items-center gap-1.5 rounded-md bg-stitch-danger text-white px-3 py-1.5 text-xs font-semibold">{icon('delete', 'text-sm')}Delete step</span></div>
  </div>
</div>"""
    return page("6 · Delete a referenced step", "6 · Deleting a step that other things depend on", behind, overlay=dialog)


# ── 7 · Traceability in context ────────────────────────────────────────
def s7_trace():
    doc = f"""
<div class="flex-1 min-w-0 space-y-3">
  {section('Main scenario', 'format_list_numbered',
           step(1, MAIN[0][0]) + step(2, MAIN[1][0], MAIN[1][1])
           + step(3, MAIN[2][0], link_chip('REQ-EXP-004'), 'origin')
           + step(4, MAIN[3][0], link_chip('REQ-EXP-007', suspect=True) + link_chip('TEST-EXP-002')))}
  <div class="rounded-lg border border-amber-300 bg-amber-50 px-4 py-2.5 text-sm text-amber-900 flex items-center gap-2">
    {icon('link_off', 'text-lg text-amber-700')}<span>{code('REQ-EXP-007')} changed since it was linked to step 4 (v3 → v4). Check the step still holds.</span>
    <span class="ml-auto flex gap-2">{btn('Show change', 'compare')}{btn('Mark reviewed', 'done')}</span>
  </div>
  {branch_card('alt', 'A3', 'Nothing selected', 3, [])}
</div>"""
    panel = f"""
<aside class="w-[400px] shrink-0 rounded-lg border border-stitch-border bg-stitch-surface shadow-stitch -my-1">
  <div class="px-4 py-3 border-b border-stitch-border flex items-center gap-2">
    {code('REQ-EXP-004', 'text-xs')}{chip('Approved', 'bg-emerald-100 text-emerald-800')}
    <span class="ml-auto flex items-center gap-2 text-stitch-muted">{icon('open_in_new', 'text-base')}{icon('close', 'text-base')}</span>
  </div>
  <div class="px-4 py-3 space-y-3 text-sm">
    <h3 class="text-base font-bold text-stitch-fg font-headline">Export a selection of requirements</h3>
    <p class="text-stitch-fg leading-relaxed">The system shall let a user export a selected subset of the project's requirements, keeping their order, codes and current versions.</p>
    <div class="grid grid-cols-2 gap-2 text-xs"><div><p class="text-stitch-muted uppercase tracking-wider text-[10px] font-bold">Category</p>Export</div><div><p class="text-stitch-muted uppercase tracking-wider text-[10px] font-bold">Reviewer</p>Dr. Sarah Smith</div></div>
    <div class="rounded-md border border-stitch-border">
      <p class="px-3 py-2 text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted border-b border-stitch-border flex items-center gap-1.5">{icon('account_tree', 'text-sm')}Used in use cases</p>
      <div class="px-3 py-2 flex items-center gap-2 bg-amber-50">{code('UC-001')}<span class="text-xs">Export requirements</span><span class="ml-auto text-[10px] font-semibold text-amber-800">step 3 · here</span></div>
      <div class="px-3 py-2 flex items-center gap-2 border-t border-stitch-border">{code('UC-002')}<span class="text-xs">Import a ReqIF package</span><span class="ml-auto text-[10px] text-stitch-muted">step 5</span></div>
    </div>
    <div class="rounded-md border border-stitch-border">
      <p class="px-3 py-2 text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted border-b border-stitch-border">Verified by</p>
      <div class="px-3 py-2 flex items-center gap-2">{code('TEST-EXP-001')}<span class="text-xs">Partial export</span>{chip('Passed', 'bg-emerald-600 text-white ml-auto')}</div>
    </div>
  </div>
</aside>"""
    body = uc_header(btn_primary('Edit', 'edit')) + f'<div class="flex gap-5 items-start">{doc}{panel}</div>'
    return page("7 · Traceability in context", "7 · A linked requirement in a side panel; a suspect link after the requirement changed", body)


# ── 8 · Structural review diff ─────────────────────────────────────────
def s8_diff():
    def badge(t, cls):
        return chip(t, cls)
    rows = [
        (1, "User opens the requirements page.", ""),
        (2, "User selects requirements and requests an export.", badge('Moved 3 → 2', 'bg-violet-100 text-violet-800')),
        (3, "System displays the available requirements.", badge('Moved 2 → 3', 'bg-violet-100 text-violet-800')),
        (4, 'System generates the file in the <del class="bg-red-100 text-red-800">chosen</del> <ins class="no-underline bg-emerald-100 text-emerald-800">selected</ins> format<ins class="no-underline bg-emerald-100 text-emerald-800"> and offers it for download</ins>.', badge('Edited', 'bg-amber-100 text-amber-800')),
        ("5", '<span class="bg-emerald-50">System records the export in the audit log.</span>', badge('Added', 'bg-emerald-100 text-emerald-800')),
    ]
    main = "".join(step(n, t, b) for n, t, b in rows)
    removed = f'<div class="flex items-start gap-2 px-1 py-1.5 opacity-70"><span class="w-4"></span><span class="mt-0.5 w-6 h-6 shrink-0 rounded-full bg-red-200 text-red-800 text-[11px] font-bold flex items-center justify-center">–</span><span class="flex-1 text-sm line-through text-red-800 leading-6">User confirms the export.</span>{badge("Removed", "bg-red-100 text-red-800")}</div>'
    branches = f"""
<div class="flex items-center gap-2 rounded-md border border-stitch-border border-l-4 border-l-sky-600 px-3 py-2 text-sm">
  <span class="font-mono text-xs font-bold">A2</span><span class="font-semibold">Nothing selected</span>
  <span class="ml-auto">{badge('Rebranched step 3 → 2', 'bg-sky-100 text-sky-800')}</span></div>
<div class="flex items-center gap-2 rounded-md border border-stitch-border border-l-4 border-l-rose-600 px-3 py-2 text-sm">
  <span class="font-mono text-xs font-bold">E4</span><span class="font-semibold">Export fails</span><span class="text-xs text-stitch-muted">· 1 step edited</span>
  <span class="ml-auto">{badge('Edited', 'bg-amber-100 text-amber-800')}</span></div>"""
    thread = f"""
<aside class="w-80 shrink-0 space-y-3">
  <div class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm px-4 py-3">
    <p class="text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted mb-2">Summary of changes</p>
    <div class="grid grid-cols-2 gap-y-1 text-xs">
      <span>{badge('Added', 'bg-emerald-100 text-emerald-800')} 1 step</span><span>{badge('Removed', 'bg-red-100 text-red-800')} 1 step</span>
      <span>{badge('Edited', 'bg-amber-100 text-amber-800')} 2 steps</span><span>{badge('Moved', 'bg-violet-100 text-violet-800')} 2 steps</span>
      <span class="col-span-2">{badge('Rebranched', 'bg-sky-100 text-sky-800')} 1 alternative</span>
    </div>
  </div>
  <div class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm">
    <p class="px-4 py-2.5 border-b border-stitch-border text-xs font-bold text-stitch-fg flex items-center gap-1.5">{icon('forum', 'text-base text-stitch-accent')}On step 4 <span class="font-normal text-stitch-muted">· 2 comments</span></p>
    <div class="px-4 py-3 space-y-3 text-sm">
      <div><p class="text-xs"><strong>Dr. Sarah Smith</strong> <span class="text-stitch-muted">· 1 h ago</span></p>
      <p>Which formats? Please link <span class="rounded-sm bg-stitch-accent/15 px-0.5 font-semibold text-stitch-accent">@alice</span> REQ-EXP-007 so the list stays in one place.</p></div>
      <div><p class="text-xs"><strong>Alice Johnson</strong> <span class="text-stitch-muted">· 20 min ago</span></p><p>Linked. The step now just says “selected format”.</p></div>
      <div class="rounded-md border border-stitch-border px-3 py-2 text-stitch-muted text-xs">Reply… (@ to mention)</div>
    </div>
  </div>
  <div class="flex gap-2">{btn('Request changes', 'undo')}<span class="inline-flex items-center gap-1.5 rounded-md bg-emerald-700 text-white px-3 py-1.5 text-xs font-semibold">{icon('check', 'text-sm')}Approve v3</span></div>
</aside>"""
    body = f"""
<div class="flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.14em] text-stitch-muted mb-2">Use cases {icon('chevron_right', 'text-sm')} <span class="text-stitch-accent">UC-001</span> {icon('chevron_right', 'text-sm')} Review</div>
<div class="flex items-center gap-3 mb-3">
  <h1 class="text-2xl font-bold text-stitch-fg font-headline">Export requirements</h1>
  {chip('In review', 'bg-sky-100 text-sky-800')}
  <span class="ml-auto flex items-center gap-2 text-sm">{btn('v2 · approved')}{icon('arrow_forward', 'text-base text-stitch-muted')}{btn('v3 · draft by Alice', None, 'border-stitch-accent text-stitch-accent')}</span>
</div>
<div class="flex gap-5">
  <div class="flex-1 min-w-0 space-y-3">
    {section('Main scenario', 'format_list_numbered', main + removed)}
    <div class="space-y-2">{branches}</div>
  </div>{thread}
</div>"""
    return page("8 · Review diff", "8 · Reviewing a new version: changes per step, comments on a step", body)


# ── 9 · Flow view ──────────────────────────────────────────────────────
def s9_flow():
    def node(x, y, w, label, text, cls="border-stitch-accent bg-stitch-surface", sel=False):
        ring = " ring-4 ring-amber-300" if sel else ""
        return (f'<div class="absolute rounded-lg border-2 {cls} shadow-sm px-3 py-2{ring}" style="left:{x}px;top:{y}px;width:{w}px">'
                f'<p class="font-mono text-[10px] font-bold text-stitch-muted">{label}</p><p class="text-xs text-stitch-fg leading-snug">{text}</p></div>')

    def pill(x, y, text, cls):
        return f'<div class="absolute rounded-full px-4 py-1.5 text-xs font-bold {cls}" style="left:{x}px;top:{y}px">{text}</div>'

    cx = 170
    svg = """
<svg class="absolute inset-0" width="100%" height="100%">
  <defs><marker id="a" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="#3949ab"/></marker>
  <marker id="r" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="#be123c"/></marker>
  <marker id="s" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="#0369a1"/></marker></defs>
  <g stroke="#3949ab" stroke-width="2" fill="none" marker-end="url(#a)">
    <line x1="170" y1="46" x2="170" y2="68"/><line x1="170" y1="134" x2="170" y2="158"/>
    <line x1="170" y1="224" x2="170" y2="248"/><line x1="170" y1="314" x2="170" y2="338"/><line x1="170" y1="404" x2="170" y2="436"/>
  </g>
  <line x1="282" y1="281" x2="356" y2="281" stroke="#0369a1" stroke-width="2" stroke-dasharray="5 4" marker-end="url(#s)"/>
  <line x1="470" y1="248" x2="470" y2="200" stroke="#0369a1" stroke-width="2" stroke-dasharray="5 4" marker-end="url(#s)"/>
  <path d="M356,160 C300,160 310,262 284,266" stroke="#0369a1" stroke-width="2" stroke-dasharray="5 4" fill="none" marker-end="url(#s)"/>
  <line x1="282" y1="371" x2="626" y2="371" stroke="#be123c" stroke-width="2" stroke-dasharray="5 4" marker-end="url(#r)"/>
  <line x1="740" y1="404" x2="740" y2="436" stroke="#be123c" stroke-width="2" marker-end="url(#r)"/>
</svg>"""
    nodes = (pill(cx - 30, 14, "Start", "bg-stitch-accent text-white")
             + node(60, 70, 220, "1", MAIN[0][0])
             + node(60, 160, 220, "2", MAIN[1][0])
             + node(60, 250, 220, "3", MAIN[2][0], sel=True)
             + node(60, 340, 220, "4", MAIN[3][0])
             + pill(cx - 70, 438, "End · file available", "bg-emerald-600 text-white")
             + node(358, 250, 230, "A3.1 · Nothing selected", "System asks the user to select at least one requirement.", "border-sky-600 bg-sky-50")
             + node(358, 130, 230, "A3.2", "User selects requirements, then back to step 3.", "border-sky-600 bg-sky-50")
             + node(628, 340, 230, "E4.1 · Export fails", "System reports the error and keeps the selection.", "border-rose-600 bg-rose-50")
             + pill(690, 438, "End · no file", "bg-rose-600 text-white"))
    legend = f"""
<aside class="w-64 shrink-0 space-y-3">
  <div class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm px-4 py-3 text-xs space-y-2">
    <p class="text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">Selected · step 3</p>
    <p class="text-stitch-fg">User selects requirements and requests an export.</p>
    <p>{link_chip('REQ-EXP-004')}</p>
    <p class="text-stitch-accent font-semibold flex items-center gap-1">{icon('article', 'text-sm')}Show in document</p>
  </div>
  <div class="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm px-4 py-3 text-xs space-y-1.5">
    <p class="text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">Legend</p>
    <p class="flex items-center gap-2"><span class="w-6 h-0.5 bg-[#3949ab]"></span>Main scenario</p>
    <p class="flex items-center gap-2"><span class="w-6 border-t-2 border-dashed border-sky-700"></span>Alternative</p>
    <p class="flex items-center gap-2"><span class="w-6 border-t-2 border-dashed border-rose-700"></span>Exception</p>
    <p class="text-stitch-muted pt-1 border-t border-stitch-border">Drawn from the same steps as the document. There is no diagram to maintain.</p>
  </div>
</aside>"""
    body = uc_header(f'<div class="flex gap-1">{btn("Document", "article")}{btn("Flow", "schema", "bg-stitch-elevated text-stitch-accent")}</div>') + f"""
<div class="flex gap-5">
  <div class="flex-1 min-w-0 rounded-lg border border-stitch-border bg-stitch-surface shadow-sm relative h-[520px] overflow-hidden"
       style="background-image: radial-gradient(rgba(0,0,0,0.08) 1px, transparent 1px); background-size: 18px 18px">
    {svg}{nodes}
    <div class="absolute right-3 bottom-3 flex gap-1">{btn('', 'zoom_in')}{btn('', 'zoom_out')}{btn('', 'fit_screen')}</div>
  </div>{legend}
</div>"""
    return page("9 · Flow view", "9 · Flow view (later iteration): derived from the same data", body)


os.makedirs(OUT, exist_ok=True)
for name, fn in [("1-list", s1_list), ("2-document", s2_document), ("3-edit", s3_edit), ("4-reorder", s4_reorder),
                 ("5-branches", s5_branches), ("6-delete", s6_delete), ("7-traceability", s7_trace),
                 ("8-review-diff", s8_diff), ("9-flow", s9_flow)]:
    with open(os.path.join(OUT, f"{name}.html"), "w") as f:
        f.write(fn())
print("ok")
