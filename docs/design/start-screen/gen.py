"""Generate the four start-screen mockups (issue: start screen after sign-in)."""
import os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "mockups")

FONTS = (
    '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Public+Sans:wght@400;600;700;800'
    '&family=Google+Sans+Text:wght@400;500;700&family=Inter:wght@300;400;500;600;700'
    '&family=JetBrains+Mono:wght@400;500;600&display=swap">'
)

PROJECTS = [
    # initial, name, slug, group, role, reqs, vers, updated, colour
    ("S", "Space Project", "space-project", "Satellite Team", "Admin", 128, 42, "2 hours ago", "bg-indigo-700"),
    ("T", "Thermal Subsystem", "thermal-subsystem", "Satellite Team", "Reviewer", 64, 23, "yesterday", "bg-teal-700"),
    ("R", "Rover Navigation", "rover-navigation", None, "Author", 37, 11, "3 days ago", "bg-amber-700"),
    ("M", "Marreq Project", "marreq-project", None, "Admin", 52, 18, "last week", "bg-sky-700"),
    ("G", "Ground Segment", "ground-segment", "Ops", "Viewer", 19, 6, "2 weeks ago", "bg-rose-700"),
    ("E", "Empty Project", "empty-project", None, "Admin", 0, 0, "3 weeks ago", "bg-slate-600"),
]
ARCHIVED = [("O", "Old Probe", "Admin"), ("L", "Launch Campaign 2025", "Author")]


def icon(name, cls="text-base"):
    return f'<span class="material-symbols-outlined {cls}" aria-hidden="true">{name}</span>'


def header():
    return f"""
<header class="h-14 shrink-0 border-b border-stitch-border bg-stitch-surface px-6 flex items-center justify-between">
  <div class="flex items-center gap-3">
    <div class="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">{icon('architecture', 'text-white text-sm')}</div>
    <div>
      <p class="text-stitch-accent text-xs uppercase tracking-wider font-bold leading-none">Marreq</p>
      <p class="text-[10px] text-stitch-muted font-mono leading-none mt-1">v0.1.17</p>
    </div>
  </div>
  <div class="flex items-center gap-4 text-stitch-muted">
    <div class="flex items-center rounded-lg border border-stitch-border p-0.5 gap-0.5">
      <span class="p-1.5 rounded-md bg-stitch-elevated text-stitch-accent">{icon('light_mode', 'text-lg')}</span>
      <span class="p-1.5">{icon('dark_mode', 'text-lg')}</span>
      <span class="p-1.5">{icon('routine', 'text-lg')}</span>
    </div>
    <span class="relative">{icon('notifications', 'text-xl')}<span class="absolute -top-1 -right-1.5 rounded-full bg-red-600 text-white text-[9px] font-bold px-1">3</span></span>
    <span class="flex items-center gap-2 text-stitch-fg">
      <span class="w-8 h-8 rounded-full bg-stitch-accent text-white text-xs font-bold flex items-center justify-center">AJ</span>
      <span class="text-sm font-semibold">Alice Johnson</span>{icon('expand_more', 'text-base text-stitch-muted')}
    </span>
  </div>
</header>"""


def page(title, body, letter):
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>{title}</title>
{FONTS}<link rel="stylesheet" href="./style.css"></head>
<body class="font-sans text-stitch">
<div class="min-h-screen flex flex-col bg-stitch-canvas">
{header()}
<main class="flex-1 px-10 py-8">{body}</main>
<footer class="shrink-0 border-t border-stitch-border bg-stitch-surface px-6 py-2 flex justify-between font-mono text-[10px] text-stitch-muted">
  <span>© 2026 Marreq</span><span>Proposal {letter} · mockup with illustrative data</span>
</footer>
</div></body></html>"""


def btn_primary(label, ic):
    return (f'<span class="inline-flex items-center gap-2 rounded-md bg-linear-to-br from-primary to-primary-container '
            f'text-white px-4 py-2 text-sm font-semibold shadow-lg">{icon(ic, "text-sm")}{label}</span>')


def btn(label, ic):
    return (f'<span class="inline-flex items-center gap-2 rounded-md border border-stitch-border bg-stitch-surface '
            f'text-stitch-fg px-4 py-2 text-sm font-semibold">{icon(ic, "text-sm text-stitch-muted")}{label}</span>')


def role_badge(role):
    colours = {
        "Admin": "bg-indigo-100 text-indigo-800",
        "Reviewer": "bg-teal-100 text-teal-800",
        "Author": "bg-amber-100 text-amber-800",
        "Viewer": "bg-slate-200 text-slate-700",
    }
    return f'<span class="rounded-sm px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider {colours[role]}">{role}</span>'


def initial(letter, colour, size="w-10 h-10 text-base"):
    return f'<span class="{size} {colour} rounded-lg text-white font-bold flex items-center justify-center shrink-0">{letter}</span>'


# ── A · Project launcher ───────────────────────────────────────────────
def option_a():
    cards = []
    for ini, name, slug, group, role, reqs, vers, updated, colour in PROJECTS:
        group_line = (f'{icon("workspaces", "text-xs")} {group}' if group else f'{icon("person", "text-xs")} Personal')
        cards.append(f"""
<div class="rounded-xl border border-stitch-border bg-stitch-surface p-4 shadow-sm hover:shadow-md flex flex-col gap-3">
  <div class="flex items-start gap-3">
    {initial(ini, colour)}
    <div class="min-w-0 flex-1">
      <p class="font-bold text-stitch-fg text-[15px] truncate">{name}</p>
      <p class="text-xs text-stitch-muted flex items-center gap-1">{group_line}</p>
    </div>
    {role_badge(role)}
  </div>
  <div class="flex gap-5 text-xs text-stitch-muted">
    <span><strong class="text-stitch-fg text-sm">{reqs}</strong> requirements</span>
    <span><strong class="text-stitch-fg text-sm">{vers}</strong> verifications</span>
  </div>
  <p class="text-[11px] text-stitch-muted border-t border-stitch-border pt-2">Updated {updated}</p>
</div>""")
    body = f"""
<div class="max-w-6xl mx-auto">
  <div class="flex items-end justify-between mb-6">
    <div>
      <p class="text-xs font-semibold uppercase tracking-wider text-stitch-muted">Your projects</p>
      <h1 class="text-2xl font-bold text-stitch-fg font-headline">Good morning, Alice</h1>
    </div>
    <div class="flex gap-2">{btn('Groups', 'workspaces')}{btn('Import bundle', 'upload_file')}{btn_primary('New project', 'add')}</div>
  </div>
  <div class="flex gap-3 mb-5">
    <div class="relative flex-1 max-w-md">
      <span class="absolute inset-y-0 left-3 flex items-center text-stitch-muted">{icon('search', 'text-sm')}</span>
      <div class="w-full pl-10 pr-4 py-2 bg-stitch-surface border border-stitch-border rounded-md text-sm text-stitch-muted">Search projects…</div>
    </div>
    <div class="flex items-center gap-2 border border-stitch-border rounded-md bg-stitch-surface px-3 text-sm">Group: <strong>All</strong>{icon('expand_more', 'text-base text-stitch-muted')}</div>
    <div class="flex items-center gap-2 border border-stitch-border rounded-md bg-stitch-surface px-3 text-sm">Sort: <strong>Recently updated</strong>{icon('expand_more', 'text-base text-stitch-muted')}</div>
  </div>
  <div class="grid grid-cols-3 gap-4">{''.join(cards)}</div>
  <div class="mt-5 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-stitch-muted">
    {icon('chevron_right', 'text-base')} Archived ({len(ARCHIVED)})
  </div>
</div>"""
    return page("A · Project launcher", body, "A")


# ── B · Workspace home ────────────────────────────────────────────────
def option_b():
    attention = [
        ("rate_review", "text-indigo-700", "REQ-PWR-012 is waiting for your review", "Space Project", "1 h ago"),
        ("link_off", "text-amber-700", "3 suspect links after REQ-THM-004 changed", "Thermal Subsystem", "5 h ago"),
        ("alternate_email", "text-sky-700", "Bob mentioned you in REQ-NAV-007", "Rover Navigation", "yesterday"),
        ("task_alt", "text-emerald-700", "VER-PWR-003 passed and closes REQ-PWR-002", "Space Project", "yesterday"),
        ("rate_review", "text-indigo-700", "REQ-THM-009 is waiting for your review", "Thermal Subsystem", "2 days ago"),
    ]
    rows = "".join(f"""
<li class="flex items-center gap-3 px-4 py-3">
  {icon(ic, 'text-xl ' + col)}
  <div class="min-w-0 flex-1"><p class="text-sm text-stitch-fg truncate">{text}</p><p class="text-[11px] text-stitch-muted">{proj}</p></div>
  <span class="text-[11px] text-stitch-muted whitespace-nowrap">{when}</span>
</li>""" for ic, col, text, proj, when in attention)
    plist = "".join(f"""
<li class="flex items-center gap-2.5 px-3 py-2 {'bg-stitch-elevated/60 rounded-md' if i == 0 else ''}">
  {initial(ini, colour, 'w-6 h-6 text-[11px]')}
  <span class="text-sm text-stitch-fg truncate flex-1">{name}</span>
  <span class="text-[10px] text-stitch-muted">{role}</span>
</li>""" for i, (ini, name, slug, group, role, *_rest, colour) in enumerate(PROJECTS))
    body = f"""
<div class="max-w-6xl mx-auto">
  <h1 class="text-2xl font-bold text-stitch-fg font-headline mb-1">Welcome back, Alice</h1>
  <p class="text-sm text-stitch-muted mb-6">You have 2 reviews and 3 suspect links waiting.</p>
  <div class="grid grid-cols-[1fr_320px] gap-6">
    <div class="space-y-6">
      <section class="rounded-xl border border-stitch-border bg-stitch-surface shadow-sm p-5">
        <p class="text-xs font-bold uppercase tracking-widest text-stitch-muted mb-3">Continue where you left off</p>
        <div class="flex items-center gap-4">
          {initial('S', 'bg-indigo-700', 'w-14 h-14 text-xl')}
          <div class="flex-1">
            <p class="text-lg font-bold text-stitch-fg">Space Project</p>
            <p class="text-sm text-stitch-muted">Requirements › REQ-PWR-012 Battery depth of discharge · 2 hours ago</p>
          </div>
          {btn_primary('Resume', 'play_arrow')}
        </div>
        <div class="flex gap-2 mt-4 pl-18">
          {btn('Dashboard', 'dashboard')}{btn('Requirements', 'list_alt')}{btn('Traceability', 'account_tree')}{btn('Reports', 'description')}
        </div>
      </section>
      <section class="rounded-xl border border-stitch-border bg-stitch-surface shadow-sm overflow-hidden">
        <div class="px-4 py-3 border-b border-stitch-border bg-stitch-elevated flex justify-between items-center">
          <p class="text-xs font-bold uppercase tracking-widest text-stitch-muted">Needs your attention</p>
          <span class="text-xs font-semibold text-stitch-accent">All notifications</span>
        </div>
        <ul class="divide-y divide-stitch-border">{rows}</ul>
      </section>
    </div>
    <aside class="rounded-xl border border-stitch-border bg-stitch-surface shadow-sm p-3 self-start">
      <div class="flex items-center justify-between px-1 mb-2">
        <p class="text-xs font-bold uppercase tracking-widest text-stitch-muted">Projects ({len(PROJECTS)})</p>
      </div>
      <div class="relative mb-2">
        <span class="absolute inset-y-0 left-2.5 flex items-center text-stitch-muted">{icon('search', 'text-sm')}</span>
        <div class="w-full pl-8 py-1.5 bg-stitch-canvas border border-stitch-border rounded-md text-xs text-stitch-muted">Filter projects…</div>
      </div>
      <ul class="space-y-0.5">{plist}</ul>
      <p class="mt-2 px-3 text-xs font-semibold uppercase tracking-wider text-stitch-muted flex items-center gap-1">{icon('chevron_right', 'text-base')} Archived ({len(ARCHIVED)})</p>
      <div class="border-t border-stitch-border mt-3 pt-3 px-1 flex flex-col gap-2 text-sm font-semibold text-stitch-fg">
        <span class="flex items-center gap-2">{icon('add_box', 'text-base text-stitch-muted')} New project</span>
        <span class="flex items-center gap-2">{icon('upload_file', 'text-base text-stitch-muted')} Import project bundle</span>
        <span class="flex items-center gap-2">{icon('workspaces', 'text-base text-stitch-muted')} Groups</span>
      </div>
    </aside>
  </div>
</div>"""
    return page("B · Workspace home", body, "B")


# ── C · Action tiles ──────────────────────────────────────────────────
def option_c():
    tiles = [
        ("play_circle", "Continue", "Space Project · Requirements", "from-primary to-primary-container text-white", True),
        ("rate_review", "Review", "4 versions waiting in 2 projects", "", False),
        ("add_box", "New project", "Personal or in a group", "", False),
        ("upload_file", "Import", "Bundle, Excel or ReqIF", "", False),
        ("workspaces", "Groups", "3 groups you belong to", "", False),
        ("admin_panel_settings", "Administration", "Users, backup, system logs", "", False),
    ]
    cells = []
    for ic, title, sub, grad, primary in tiles:
        if primary:
            cls = f"bg-linear-to-br {grad} border-transparent shadow-lg"
            tcol, scol, icol = "text-white", "text-white/80", "text-white"
        else:
            cls = "bg-stitch-surface border-stitch-border shadow-sm"
            tcol, scol, icol = "text-stitch-fg", "text-stitch-muted", "text-stitch-accent"
        cells.append(f"""
<div class="rounded-2xl border {cls} p-6 h-40 flex flex-col justify-between">
  {icon(ic, 'text-4xl ' + icol)}
  <div><p class="text-lg font-bold {tcol}">{title}</p><p class="text-sm {scol}">{sub}</p></div>
</div>""")
    body = f"""
<div class="max-w-4xl mx-auto pt-4">
  <p class="text-center text-xs font-semibold uppercase tracking-wider text-stitch-muted">Signed in as Alice Johnson</p>
  <h1 class="text-center text-3xl font-bold text-stitch-fg font-headline mb-8">What do you want to do?</h1>
  <div class="grid grid-cols-3 gap-5">{''.join(cells)}</div>
  <div class="mt-8 rounded-xl border border-stitch-border bg-stitch-surface p-4 flex items-center gap-3 shadow-sm">
    {icon('folder_open', 'text-xl text-stitch-muted')}
    <span class="text-sm font-semibold text-stitch-fg">Open a project</span>
    <div class="flex-1 flex items-center justify-between border border-stitch-border rounded-md bg-stitch-canvas px-3 py-2 text-sm">
      <span class="flex items-center gap-2">{initial('S', 'bg-indigo-700', 'w-5 h-5 text-[10px]')} Space Project <span class="text-stitch-muted">· Satellite Team</span></span>
      {icon('expand_more', 'text-base text-stitch-muted')}
    </div>
    {btn_primary('Open', 'arrow_forward')}
  </div>
</div>"""
    return page("C · Action tiles", body, "C")


# ── D · Search-first ──────────────────────────────────────────────────
def option_d():
    recent = [
        ("S", "Space Project", "bg-indigo-700", "Requirements › REQ-PWR-012", "2 hours ago"),
        ("T", "Thermal Subsystem", "bg-teal-700", "Traceability › Matrix", "yesterday"),
        ("R", "Rover Navigation", "bg-amber-700", "Dashboard", "3 days ago"),
        ("M", "Marreq Project", "bg-sky-700", "Baselines › Release 1.0", "last week"),
    ]
    rows = "".join(f"""
<li class="flex items-center gap-3 px-4 py-2.5 {'bg-stitch-elevated/70' if i == 0 else ''}">
  {initial(ini, col, 'w-7 h-7 text-xs')}
  <span class="text-sm font-semibold text-stitch-fg w-48">{name}</span>
  <span class="text-sm text-stitch-muted flex-1">{where}</span>
  <span class="text-xs text-stitch-muted">{when}</span>
  {'<kbd class="ml-2 rounded-sm border border-stitch-border bg-stitch-surface px-1.5 text-[10px] font-mono text-stitch-muted">↵</kbd>' if i == 0 else '<span class="ml-2 w-6"></span>'}
</li>""" for i, (ini, name, col, where, when) in enumerate(recent))
    body = f"""
<div class="max-w-3xl mx-auto pt-10">
  <h1 class="text-center text-2xl font-bold text-stitch-fg font-headline mb-6">Where to, Alice?</h1>
  <div class="rounded-xl border-2 border-stitch-accent/60 bg-stitch-surface shadow-lg flex items-center gap-3 px-5 py-4">
    {icon('search', 'text-2xl text-stitch-accent')}
    <span class="flex-1 text-base text-stitch-muted">Jump to a project, a requirement (REQ-…), or an action…</span>
    <kbd class="rounded-sm border border-stitch-border bg-stitch-canvas px-2 py-0.5 text-xs font-mono text-stitch-muted">Ctrl K</kbd>
  </div>
  <p class="text-center text-xs text-stitch-muted mt-2">Try “thermal”, “REQ-PWR-012” or “new project”.</p>
  <section class="mt-10">
    <p class="text-xs font-bold uppercase tracking-widest text-stitch-muted mb-2 px-1">Recent</p>
    <ul class="rounded-xl border border-stitch-border bg-stitch-surface shadow-sm overflow-hidden divide-y divide-stitch-border">{rows}</ul>
  </section>
  <div class="mt-6 flex items-center justify-between px-1 text-sm">
    <div class="flex gap-5 font-semibold text-stitch-fg">
      <span class="flex items-center gap-1.5">{icon('add_box', 'text-base text-stitch-muted')} New project</span>
      <span class="flex items-center gap-1.5">{icon('upload_file', 'text-base text-stitch-muted')} Import</span>
      <span class="flex items-center gap-1.5">{icon('workspaces', 'text-base text-stitch-muted')} Groups</span>
    </div>
    <div class="flex gap-4 text-stitch-accent font-semibold">
      <span>All projects ({len(PROJECTS)})</span><span class="text-stitch-muted">Archived ({len(ARCHIVED)})</span>
    </div>
  </div>
</div>"""
    return page("D · Search-first", body, "D")


# ── E · Search with attention (D's minimal layout + B's attention list) ──
def option_e():
    attention = [
        ("rate_review", "text-indigo-700", "REQ-PWR-012 is waiting for your review", "S", "bg-indigo-700", "Space Project", "1 h"),
        ("link_off", "text-amber-700", "3 suspect links after REQ-THM-004 changed", "T", "bg-teal-700", "Thermal Subsystem", "5 h"),
        ("alternate_email", "text-sky-700", "Bob mentioned you in REQ-NAV-007", "R", "bg-amber-700", "Rover Navigation", "1 d"),
        ("rate_review", "text-indigo-700", "REQ-THM-009 is waiting for your review", "T", "bg-teal-700", "Thermal Subsystem", "2 d"),
    ]
    att_rows = "".join(f"""
<li class="flex items-center gap-3 px-4 py-2.5">
  {icon(ic, 'text-lg ' + col)}
  <span class="text-sm text-stitch-fg flex-1 truncate">{text}</span>
  <span class="flex items-center gap-1.5 text-xs text-stitch-muted w-40">{initial(ini, pcol, 'w-4 h-4 text-[9px] rounded-sm')}{proj}</span>
  <span class="text-xs text-stitch-muted w-8 text-right">{when}</span>
  {icon('chevron_right', 'text-base text-stitch-muted')}
</li>""" for ic, col, text, ini, pcol, proj, when in attention)
    recent = [
        ("S", "Space Project", "bg-indigo-700", "Requirements › REQ-PWR-012", "2 hours ago"),
        ("T", "Thermal Subsystem", "bg-teal-700", "Traceability › Matrix", "yesterday"),
        ("R", "Rover Navigation", "bg-amber-700", "Dashboard", "3 days ago"),
    ]
    rec_rows = "".join(f"""
<li class="flex items-center gap-3 px-4 py-2.5 {'bg-stitch-elevated/70' if i == 0 else ''}">
  {initial(ini, col, 'w-6 h-6 text-[11px]')}
  <span class="text-sm font-semibold text-stitch-fg w-44">{name}</span>
  <span class="text-sm text-stitch-muted flex-1">{where}</span>
  <span class="text-xs text-stitch-muted">{when}</span>
  {'<kbd class="ml-2 rounded-sm border border-stitch-border bg-stitch-surface px-1.5 text-[10px] font-mono text-stitch-muted">↵</kbd>' if i == 0 else '<span class="ml-2 w-6"></span>'}
</li>""" for i, (ini, name, col, where, when) in enumerate(recent))
    body = f"""
<div class="max-w-3xl mx-auto pt-2">
  <h1 class="text-center text-2xl font-bold text-stitch-fg font-headline">Where to, Alice?</h1>
  <p class="text-center text-sm text-stitch-muted mt-1 mb-5">2 reviews and 3 suspect links are waiting for you.</p>
  <div class="rounded-xl border-2 border-stitch-accent/60 bg-stitch-surface shadow-lg flex items-center gap-3 px-5 py-3.5">
    {icon('search', 'text-2xl text-stitch-accent')}
    <span class="flex-1 text-base text-stitch-muted">Jump to a project, a requirement (REQ-…), or an action…</span>
    <kbd class="rounded-sm border border-stitch-border bg-stitch-canvas px-2 py-0.5 text-xs font-mono text-stitch-muted">Ctrl K</kbd>
  </div>
  <section class="mt-8">
    <div class="flex items-baseline justify-between px-1 mb-2">
      <p class="text-xs font-bold uppercase tracking-widest text-stitch-muted">Needs your attention <span class="ml-1 rounded-full bg-red-600 text-white px-1.5 py-0.5 text-[10px] tracking-normal">5</span></p>
      <span class="text-xs font-semibold text-stitch-accent">Show all</span>
    </div>
    <ul class="rounded-xl border border-stitch-border bg-stitch-surface shadow-sm overflow-hidden divide-y divide-stitch-border">{att_rows}</ul>
  </section>
  <section class="mt-6">
    <p class="text-xs font-bold uppercase tracking-widest text-stitch-muted mb-2 px-1">Recent</p>
    <ul class="rounded-xl border border-stitch-border bg-stitch-surface shadow-sm overflow-hidden divide-y divide-stitch-border">{rec_rows}</ul>
  </section>
  <div class="mt-5 flex items-center justify-between px-1 text-sm">
    <div class="flex gap-5 font-semibold text-stitch-fg">
      <span class="flex items-center gap-1.5">{icon('add_box', 'text-base text-stitch-muted')} New project</span>
      <span class="flex items-center gap-1.5">{icon('upload_file', 'text-base text-stitch-muted')} Import</span>
      <span class="flex items-center gap-1.5">{icon('workspaces', 'text-base text-stitch-muted')} Groups</span>
    </div>
    <div class="flex gap-4 text-stitch-accent font-semibold">
      <span>All projects ({len(PROJECTS)})</span><span class="text-stitch-muted">Archived ({len(ARCHIVED)})</span>
    </div>
  </div>
</div>"""
    return page("E · Search with attention", body, "E")


os.makedirs(OUT, exist_ok=True)
for name, fn in [("A-launcher", option_a), ("B-workspace", option_b), ("C-tiles", option_c), ("D-search", option_d), ("E-search-attention", option_e)]:
    with open(os.path.join(OUT, f"{name}.html"), "w") as f:
        f.write(fn())
print("ok")
