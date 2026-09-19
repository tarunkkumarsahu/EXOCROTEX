import { useCallback, useEffect, useMemo, useState, type FormEvent, type ReactNode } from 'react'
import { Activity, ArrowDownRight, ArrowRight, BookOpen, BrainCircuit, Check, ChevronRight, CircleHelp, Clock3, Database, ExternalLink, Eye, FileClock, GitBranch, Layers, LockKeyhole, Menu, Plus, RefreshCw, Search, ShieldCheck, Trash2, X } from 'lucide-react'
import AIChat from './AIChat'
import { api, type CognitiveEvent, type MemoryKind, type MemoryConflict, type Observation, type Status, type StoredMemory, type WorkingFact } from './api'

type Page = 'overview' | 'chat' | 'memory' | 'evidence' | 'connections'
type Selection = { title: string; event?: CognitiveEvent; conflicts?: MemoryConflict[] }

function formatDate(value: string): string {
  return new Date(value).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
}

function ShortId({ value }: { value: string }) { return <code title={value}>{value.slice(0, 8)}…</code> }
function Pill({ children, tone = 'neutral' }: { children: ReactNode; tone?: 'neutral' | 'green' | 'amber' | 'red' | 'blue' }) {
  return <span className={`pill pill-${tone}`}>{children}</span>
}
function Empty({ title, detail }: { title: string; detail: string }) {
  return <div className="empty"><Layers size={25} strokeWidth={1.4} /><strong>{title}</strong><span>{detail}</span></div>
}

export default function App() {
  const [page, setPage] = useState<Page>('overview')
  const [menuOpen, setMenuOpen] = useState(false)
  const [memories, setMemories] = useState<StoredMemory[]>([])
  const [facts, setFacts] = useState<WorkingFact[]>([])
  const [status, setStatus] = useState<Status | null>(null)
  const [online, setOnline] = useState(false)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [search, setSearch] = useState('')
  const [entitySearch, setEntitySearch] = useState('')
  const [memoryText, setMemoryText] = useState('')
  const [memoryKind, setMemoryKind] = useState<MemoryKind>('Semantic')
  const [memoryTopic, setMemoryTopic] = useState('')
  const [addOpen, setAddOpen] = useState(false)
  const [edit, setEdit] = useState<StoredMemory | null>(null)
  const [editText, setEditText] = useState('')
  const [selection, setSelection] = useState<Selection | null>(null)
  const [obsForm, setObsForm] = useState({ source_key: '', version: '1', value: '' })
  const [latestObs, setLatestObs] = useState<Observation | null>(null)
  const [factForm, setFactForm] = useState({ entity: '', attribute: '', value: '', kind: 'Observed' as 'Observed' | 'Derived', evidence: '' })

  const refresh = useCallback(async () => {
    try {
      const [current, stored, working] = await Promise.all([api.status(), api.memories(''), api.facts('')])
      setStatus(current)
      setMemories(stored)
      setFacts(working)
      setOnline(true)
      setError('')
    } catch (caught) {
      setOnline(false)
      setError(caught instanceof Error ? caught.message : 'Could not load the cognitive workspace')
    } finally { setLoading(false) }
  }, [])

  useEffect(() => { void refresh() }, [refresh])
  const visibleMemories = useMemo(() => memories.filter(({ memory, topic_key }) =>
    [memory.text, memory.kind, memory.epistemic_type, topic_key ?? ''].join(' ').toLowerCase().includes(search.toLowerCase().trim()),
  ), [memories, search])
  const visibleFacts = useMemo(() => facts.filter(f =>
    [f.entity, f.attribute, f.value, f.status].join(' ').toLowerCase().includes(entitySearch.toLowerCase().trim()),
  ), [facts, entitySearch])
  const staleCount = facts.filter(f => f.status === 'Stale').length

  async function action(work: () => Promise<void>, message: string) {
    setBusy(true); setError(''); setNotice('')
    try { await work(); await refresh(); setNotice(message) }
    catch (caught) { setError(caught instanceof Error ? caught.message : 'The operation failed') }
    finally { setBusy(false) }
  }

  function navigate(next: Page) { setPage(next); setMenuOpen(false); setError(''); setNotice('') }

  async function saveMemory(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!memoryText.trim()) return
    await action(async () => {
      await api.remember(memoryText.trim(), memoryKind, memoryTopic.trim())
      setMemoryText(''); setMemoryTopic(''); setAddOpen(false)
    }, 'Memory saved in your local cognitive database.')
  }

  async function correctMemory(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!edit || !editText.trim()) return
    await action(async () => {
      await api.correct(edit.memory.id, editText.trim()); setEdit(null)
    }, 'Correction saved as a new revision; old revision superseded.')
  }

  function confirmForget(record: StoredMemory) {
    const yes = window.confirm(`Redact this memory and its correction history?\n\n${record.memory.text}\n\nExisting backups are not affected.`)
    if (!yes) return
    void action(async () => { await api.forget(record.memory.id); setSelection(null) }, 'Memory and its local revision chain redacted.')
  }

  function showSource(record: StoredMemory) {
    void action(async () => {
      const event = await api.source(record.memory.source_event_id)
      setSelection({ title: 'Source & provenance', event })
    }, '')
  }

  function showConflicts(record: StoredMemory) {
    void action(async () => {
      const conflicts = await api.conflicts(record.memory.id)
      setSelection({ title: 'Potential conflicts', conflicts })
    }, '')
  }

  async function recordObservation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const version = Number(obsForm.version)
    if (!Number.isSafeInteger(version) || version < 1) { setError('Version must be a positive whole number'); return }
    await action(async () => {
      const observation = await api.observe(obsForm.source_key, version, obsForm.value)
      setLatestObs(observation)
      setObsForm({ ...obsForm, version: String(version + 1), value: '' })
    }, 'Observation recorded. Older facts depending on this source may now be stale.')
  }

  async function createFact(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const ids = factForm.evidence.split(',').map(x => x.trim()).filter(Boolean)
    if (!ids.length) { setError('A working fact must cite at least one observation ID'); return }
    await action(async () => {
      await api.addFact(factForm.entity, factForm.attribute, factForm.value, factForm.kind, ids)
      setFactForm({ entity: '', attribute: '', value: '', kind: 'Observed', evidence: '' })
    }, 'Evidence-linked working fact created.')
  }

  return <div className="app-shell">
    <div className="ambient ambient-one" /><div className="ambient ambient-two" />
    <aside className={`sidebar ${menuOpen ? 'mobile-open' : ''}`}>
      <div className="brand"><span className="brand-icon"><BrainCircuit size={23}/></span><div><strong>EXOCORTEX</strong><small>COGNITIVE SYSTEMS</small></div></div>
      <div className="sidebar-label">WORKSPACE</div>
      <nav aria-label="Main navigation">
        {([
          ['overview', Activity, 'Overview'],
          ['chat', BrainCircuit, 'Cognitive chat'],
          ['memory', Database, 'Memory vault'],
          ['evidence', GitBranch, 'Evidence & facts'],
          ['connections', Layers, 'Connections'],
        ] as const).map(([key, Icon, label]) =>
          <button key={key} className={`nav-button ${page === key ? 'selected' : ''}`} onClick={() => navigate(key)}><Icon size={18}/><span>{label}</span>{page === key && <span className="nav-indicator"/>}</button>
        )}
      </nav>
      <div className="sidebar-spacer" />
      <div className="runtime-box"><div className="runtime-top"><span className={`pulse ${online ? '' : 'offline'}`} /> <span>{online ? 'KERNEL ONLINE' : 'API OFFLINE'}</span></div><span>Rust engine · SQLite memory</span><span>Local-first · V0.7</span></div>
      <div className="sidebar-bottom"><LockKeyhole size={15}/> Local-first research prototype</div>
    </aside>

    {menuOpen && <button aria-label="Close navigation" className="mobile-backdrop" onClick={() => setMenuOpen(false)}/>}
    <main className="main">
      <header className="topbar"><button className="mobile-menu icon-button" aria-label="Open navigation" onClick={() => setMenuOpen(true)}><Menu size={20}/></button><div className="breadcrumbs">WORKSPACE <ChevronRight size={13}/> <span>{page === 'overview' ? 'Overview' : page === 'chat' ? 'Cognitive chat' : page === 'memory' ? 'Memory vault' : page === 'evidence' ? 'Evidence & facts' : 'Connections'}</span></div><div className="topbar-right"><span className="topbar-version">V0.7 LOCAL AI</span><button className="icon-button refresh-button" title="Refresh live database state" aria-label="Refresh" onClick={() => { setLoading(true); void refresh() }}><RefreshCw size={16}/></button><div className="user-avatar">EX</div></div></header>
      <div className="content">
        {error && <div className="banner error" role="alert"><CircleHelp size={17}/><span>{error}</span><button onClick={() => setError('')} aria-label="Dismiss error"><X size={15}/></button></div>}
        {notice && <div className="banner notice" role="status"><Check size={17}/><span>{notice}</span><button onClick={() => setNotice('')} aria-label="Dismiss message"><X size={15}/></button></div>}
        {page === 'chat' && <AIChat/>}
        {page === 'overview' && <>
          <div className="hero"><div className="hero-copy"><span className="eyebrow"><span className="green-dot"/> YOUR PERSONAL COGNITIVE WORKSPACE</span><h1>Think bigger.<br/><em>Remember everything that matters.</em></h1><p>One place to inspect your memory, trace evidence and carry context across sessions. Your decisions stay yours.</p><div className="hero-actions"><button className="primary" onClick={() => { navigate('memory'); setAddOpen(true) }}><Plus size={17}/> Capture a memory</button><button className="ghost-button" onClick={() => navigate('evidence')}>Inspect evidence <ArrowRight size={17}/></button></div></div><div className="hero-art" aria-hidden="true"><div className="orbit orbit-a"/><div className="orbit orbit-b"/><div className="orbit orbit-c"/><div className="core-brain"><BrainCircuit size={56} strokeWidth={1.15}/></div><span className="orbit-node n-one"/><span className="orbit-node n-two"/><span className="orbit-node n-three"/></div></div>
          <div className="section-heading"><div><span className="eyebrow">LIVE SYSTEM</span><h2>Your cognitive state</h2></div><span className="section-description">Directly from your local SQLite database</span></div>
          <div className="metric-grid">
            <Metric icon={<Database size={20}/>} label="Active memories" value={status?.memories} detail="Persistent knowledge" />
            <Metric icon={<GitBranch size={20}/>} label="Evidence records" value={status?.observations} detail="Versioned observations" />
            <Metric icon={<Layers size={20}/>} label="Working facts" value={status?.facts} detail={`${staleCount} currently marked stale`} />
            <Metric icon={<Activity size={20}/>} label="Recorded events" value={status?.events} detail="Provenance & activity" />
          </div>
          <div className="overview-grid"><section className="panel recent-panel"><div className="panel-head"><div><span className="eyebrow">KNOWLEDGE</span><h3>Recent memories</h3></div><button className="inline-action" onClick={() => navigate('memory')}>View all <ArrowRight size={15}/></button></div>{memories.length ? memories.slice(0,4).map(m => <div className="recent-entry" key={m.memory.id}><div className="recent-icon"><BookOpen size={16}/></div><div><strong>{m.memory.text}</strong><span>{m.memory.kind} · {formatDate(m.memory.created_at)}</span></div><ChevronRight size={16} className="dim"/></div>) : <Empty title="No saved context yet" detail="Capture your first memory to build a persistent knowledge base."/>}</section>
            <section className="panel trust-panel"><div className="panel-head"><div><span className="eyebrow">SYSTEM PRINCIPLES</span><h3>Built around your control</h3></div><ShieldCheck size={22} className="accent-icon"/></div><div className="trust-item"><span className="trust-icon"><Database size={18}/></span><div><strong>Persistent by design</strong><p>Memory survives application restarts in your local database.</p></div></div><div className="trust-item"><span className="trust-icon"><GitBranch size={18}/></span><div><strong>Evidence before assertion</strong><p>Inspect where records came from and whether working facts have gone stale.</p></div></div><div className="trust-item"><span className="trust-icon"><LockKeyhole size={18}/></span><div><strong>You remain in control</strong><p>Model actions and JARVIS integrations are not connected yet.</p></div></div></section></div>
        </>}

        {page === 'memory' && <>
          <div className="page-heading"><div><span className="eyebrow">YOUR EXTERNAL MEMORY</span><h1>Memory vault</h1><p>Search, capture and correct your persistent knowledge. All records come from the Rust cognitive kernel.</p></div><button className="primary" onClick={() => setAddOpen(true)}><Plus size={17}/> New memory</button></div>
          <section className="panel"><div className="table-toolbar"><div className="search-box"><Search size={18}/><input value={search} onChange={e => setSearch(e.target.value)} placeholder="Search memories, topics or types…" aria-label="Search memories"/></div><span className="small-muted">{visibleMemories.length} of {memories.length} memories</span></div><div className="memory-list">{visibleMemories.length ? visibleMemories.map(record => <div className="memory-row" key={record.memory.id}><span className="memory-bullet"><BookOpen size={18}/></span><div className="memory-main"><div className="memory-meta"><Pill tone="blue">{record.memory.kind}</Pill><Pill tone="green">{record.memory.epistemic_type}</Pill>{record.topic_key && <Pill>{record.topic_key}</Pill>}</div><p>{record.memory.text}</p><div className="memory-secondary"><Clock3 size={13}/>{formatDate(record.memory.created_at)} <span className="dot-separator">·</span> Source <ShortId value={record.memory.source_event_id}/></div></div><div className="row-actions"><button className="icon-button" title="Inspect source" aria-label="Inspect source" onClick={() => showSource(record)}><Eye size={17}/></button><button className="icon-button" title="Correct memory" aria-label="Correct memory" onClick={() => { setEdit(record); setEditText(record.memory.text) }}><RefreshCw size={16}/></button>{record.topic_key && <button className="icon-button" title="Check topic conflicts" aria-label="Check topic conflicts" onClick={() => showConflicts(record)}><GitBranch size={16}/></button>}<button className="icon-button danger-icon" title="Forget memory" aria-label="Forget memory" onClick={() => confirmForget(record)}><Trash2 size={16}/></button></div></div>) : <Empty title="No matching memories" detail="Change your search or capture a new memory."/>}</div></section>
        </>}

        {page === 'evidence' && <>
          <div className="page-heading"><div><span className="eyebrow">EVIDENCE-LINKED WORKING STATE</span><h1>Evidence & facts</h1><p>Record source snapshots, create cited working facts and inspect evidence freshness. Observations are manually reported in this version.</p></div><Pill tone="amber">Manual input · V0.3 core</Pill></div>
          <div className="evidence-grid"><section className="panel form-panel"><div className="panel-head"><div><span className="eyebrow">01 / DATA INTAKE</span><h3>Record an observation</h3></div><Database size={19} className="accent-icon"/></div><form onSubmit={recordObservation} className="stack-form"><label>Source key<input required maxLength={120} value={obsForm.source_key} onChange={e => setObsForm({ ...obsForm, source_key: e.target.value })} placeholder="e.g. repo-main"/></label><label>Source version<input required type="number" min="1" step="1" value={obsForm.version} onChange={e => setObsForm({ ...obsForm, version: e.target.value })}/></label><label>Reported value<textarea required maxLength={8000} value={obsForm.value} onChange={e => setObsForm({ ...obsForm, value: e.target.value })} placeholder="e.g. commit-B" rows={3}/></label><button className="primary fill" type="submit" disabled={busy}>Record observation <ArrowRight size={16}/></button></form>{latestObs && <div className="result-box"><strong><Check size={14}/> Most recent observation saved</strong><p><ShortId value={latestObs.id}/> · {latestObs.source_key} · v{latestObs.version}</p><button className="inline-action" onClick={() => { navigator.clipboard?.writeText(latestObs.id).catch(() => {}); setFactForm(f => ({ ...f, evidence: latestObs.id })) }}>Use ID in fact form <ArrowDownRight size={14}/></button></div>}</section>
            <section className="panel form-panel"><div className="panel-head"><div><span className="eyebrow">02 / KNOWLEDGE CONSTRUCTION</span><h3>Add a working fact</h3></div><GitBranch size={19} className="accent-icon"/></div><form onSubmit={createFact} className="stack-form"><div className="two-fields"><label>Entity<input required maxLength={120} value={factForm.entity} onChange={e => setFactForm({ ...factForm, entity: e.target.value })} placeholder="project"/></label><label>Attribute<input required maxLength={120} value={factForm.attribute} onChange={e => setFactForm({ ...factForm, attribute: e.target.value })} placeholder="commit"/></label></div><label>Claim value<input required maxLength={8000} value={factForm.value} onChange={e => setFactForm({ ...factForm, value: e.target.value })} placeholder="commit-B"/></label><label>Evidence observation UUID(s)<input required value={factForm.evidence} onChange={e => setFactForm({ ...factForm, evidence: e.target.value })} placeholder="UUID, UUID…"/></label><label>Fact classification<select value={factForm.kind} onChange={e => setFactForm({ ...factForm, kind: e.target.value as 'Observed' | 'Derived' })}><option>Observed</option><option>Derived</option></select></label><button className="secondary-button fill" type="submit" disabled={busy}>Create evidence-linked fact <ArrowRight size={16}/></button></form></section></div>
          <section className="panel facts-panel"><div className="panel-head"><div><span className="eyebrow">WORKING STATE</span><h3>Facts and freshness</h3></div><Pill tone={staleCount ? 'amber' : 'green'}>{staleCount} stale</Pill></div><div className="table-toolbar"><div className="search-box"><Search size={17}/><input value={entitySearch} onChange={e => setEntitySearch(e.target.value)} placeholder="Filter entity, status or value…" aria-label="Search working facts"/></div><span className="small-muted">{visibleFacts.length} facts</span></div><div className="facts-list">{visibleFacts.length ? visibleFacts.map(f => <div className="fact-row" key={f.id}><div><div className="memory-meta"><Pill tone={f.status === 'Stale' ? 'amber' : f.status === 'Disputed' ? 'red' : 'green'}>{f.status}</Pill><Pill>{f.kind}</Pill></div><strong>{f.entity}.{f.attribute}</strong><p>{f.value}</p><div className="memory-secondary">Evidence: {f.evidence_ids.map(id => <ShortId key={id} value={id}/>)}</div></div><time>{formatDate(f.created_at)}</time></div>) : <Empty title="No working facts to show" detail="Record a source observation, then create a working fact citing its UUID."/>}</div></section>
        </>}

        {page === 'connections' && <><div className="page-heading"><div><span className="eyebrow">FUTURE CONNECTIVITY</span><h1>Connected intelligence</h1><p>EXOCORTEX has a working local API. External model integrations, automation and JARVIS remain separate milestones.</p></div></div><div className="connection-grid"><Connection icon={<BrainCircuit size={22}/>} title="Local cognitive chat" detail="Ollama model adapter now available in the desktop Cognitive chat section. Reasoning and citations remain experimental."/><Connection icon={<Layers size={22}/>} title="Tauri desktop" detail="Native Windows workspace connects directly to cognitive-core and uses local SQLite."/><Connection icon={<Activity size={22}/>} title="JARVIS bridge" detail="Permission-aware integration with Jarvis while both systems remain independently usable."/></div><div className="panel connection-note"><ShieldCheck size={19}/><div><strong>No external connectors are enabled.</strong><p>This interface does not pretend that AI, GitHub, email or JARVIS are operating. Real integrations will require explicit permissions and separate tests.</p></div></div></>}
      </div>
      <footer className="footer"><span>EXOCORTEX · Human cognitive extension research</span><span><span className={`pulse ${online ? '' : 'offline'}`}/> {online ? 'Connected to Rust API' : loading ? 'Connecting…' : 'Disconnected'}</span></footer>
    </main>

    {(addOpen || edit || selection) && <div className="modal-backdrop" onMouseDown={event => { if (event.target === event.currentTarget) { setAddOpen(false); setEdit(null); setSelection(null) } }}><section className="modal" role="dialog" aria-modal="true" aria-label={selection?.title ?? (edit ? 'Correct memory' : 'Capture memory')}><div className="modal-head"><div><span className="eyebrow">COGNITIVE MEMORY</span><h3>{selection?.title ?? (edit ? 'Correct existing memory' : 'Capture a new memory')}</h3></div><button className="icon-button" aria-label="Close dialog" onClick={() => { setAddOpen(false); setEdit(null); setSelection(null) }}><X size={19}/></button></div>{selection ? <div className="modal-body">{selection.event && <><div className="detail-item"><span>Source event</span><ShortId value={selection.event.id}/></div><div className="detail-item"><span>Event type</span><strong>{selection.event.kind}</strong></div><div className="detail-item"><span>Recorded</span><strong>{formatDate(selection.event.timestamp)}</strong></div><div className="detail-content">{selection.event.content}</div></>}{selection.conflicts && (selection.conflicts.length ? selection.conflicts.map(c => <div className="detail-content" key={c.other_id}><Pill tone="amber">{c.topic_key}</Pill><p>{c.other_text}</p><ShortId value={c.other_id}/></div>) : <p className="small-muted">No different active claims under this explicit topic key.</p>)}</div> : edit ? <form className="stack-form modal-body" onSubmit={correctMemory}><p className="small-muted">This creates a new current revision. The previous memory remains in the correction history.</p><label>Replacement text<textarea rows={5} maxLength={8000} required value={editText} onChange={e => setEditText(e.target.value)}/></label><button type="submit" disabled={busy || !editText.trim()} className="primary fill">Save correction <Check size={17}/></button></form> : <form className="stack-form modal-body" onSubmit={saveMemory}><label>What should EXOCORTEX remember?<textarea rows={5} maxLength={8000} required autoFocus value={memoryText} onChange={e => setMemoryText(e.target.value)} placeholder="A project decision, observation or commitment…"/></label><label>Memory type<select value={memoryKind} onChange={e => setMemoryKind(e.target.value as MemoryKind)}><option>Semantic</option><option>Episodic</option><option>Working</option><option>Prospective</option></select></label><label>Topic key <span className="small-muted">(optional)</span><input maxLength={120} value={memoryTopic} onChange={e => setMemoryTopic(e.target.value)} placeholder="e.g. exocortex-roadmap"/></label><p className="small-muted">This is saved as a user-confirmed statement, not independently verified external truth.</p><button type="submit" disabled={busy || !memoryText.trim()} className="primary fill">Save memory <ArrowRight size={17}/></button></form>}</section></div>}
  </div>
}

function Metric({ icon, label, value, detail }: { icon: ReactNode; label: string; value: number | undefined; detail: string }) {
  return <div className="metric panel"><div className="metric-top"><span className="metric-icon">{icon}</span><ArrowDownRight size={17} className="metric-arrow"/></div><div className="metric-value">{value ?? '—'}</div><strong>{label}</strong><p>{detail}</p></div>
}
function Connection({ icon, title, detail }: { icon: ReactNode; title: string; detail: string }) {
  return <div className="panel connection-card"><div className="connection-icon">{icon}</div><Pill tone="amber">Planned · Not connected</Pill><h3>{title}</h3><p>{detail}</p></div>
}
