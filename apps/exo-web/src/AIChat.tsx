import { useEffect, useRef, useState, type FormEvent } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { ArrowRight, BrainCircuit, ChevronDown, Database, MessageCircle, RefreshCw, Send, ShieldCheck, Trash2, WifiOff } from 'lucide-react'
import './ai-chat.css'

interface ModelCatalog {
  available: boolean
  models: string[]
  detail: string
}
interface Source {
  label: string
  id: string
  source_event_id: string | null
  kind: string
  status: string
  text: string
}
interface Reply {
  answer: string
  model: string
  sources: Source[]
}
interface ChatItem {
  id: number
  role: 'user' | 'assistant'
  content: string
  sources?: Source[]
  model?: string
}
const inDesktop = isTauri()

export default function AIChat() {
  const [catalog, setCatalog] = useState<ModelCatalog | null>(null)
  const [model, setModel] = useState('')
  const [question, setQuestion] = useState('')
  const [messages, setMessages] = useState<ChatItem[]>([])
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [checking, setChecking] = useState(false)
  const [expanded, setExpanded] = useState<Record<number, boolean>>({})
  const end = useRef<HTMLDivElement>(null)
  const counter = useRef(0)

  async function checkModels() {
    if (!inDesktop) return
    setChecking(true)
    setError('')
    try {
      const next = await invoke<ModelCatalog>('ai_models')
      setCatalog(next)
      setModel(current => next.models.includes(current) ? current : next.models[0] ?? '')
    } catch (caught) {
      setCatalog({ available: false, models: [], detail: 'Could not reach the native model adapter.' })
      setError(String(caught))
    } finally {
      setChecking(false)
    }
  }

  useEffect(() => { void checkModels() }, [])
  useEffect(() => { end.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' }) }, [messages, busy])

  async function send(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    const text = question.trim()
    if (!text || !model || busy || text.length > 2000) return
    const history = messages.slice(-6).map(({ role, content }) => ({ role, content: content.slice(0, 1000) }))
    const mine: ChatItem = { id: ++counter.current, role: 'user', content: text }
    setMessages(previous => [...previous, mine])
    setQuestion('')
    setBusy(true)
    setError('')
    try {
      const reply = await invoke<Reply>('ai_chat', { question: text, model, history })
      const theirs: ChatItem = {
        id: ++counter.current,
        role: 'assistant',
        content: reply.answer,
        sources: reply.sources,
        model: reply.model,
      }
      setMessages(previous => [...previous, theirs])
    } catch (caught) {
      setError('No AI answer was generated: ' + String(caught))
    } finally {
      setBusy(false)
    }
  }

  return <div className="ai-workspace">
    <div className="page-heading ai-page-heading">
      <div>
        <span className="eyebrow">V0.7 / LOCAL REASONING</span>
        <h1>Cognitive chat</h1>
        <p>Ask about your saved projects and evidence. The AI receives a small, relevant slice of your existing local memory.</p>
      </div>
      <div className="ai-provider"><span className={'pulse ' + (catalog?.available ? '' : 'offline')} />
        {catalog?.available ? 'LOCAL MODEL SERVICE' : 'MODEL NOT CONNECTED'}
      </div>
    </div>
    {!inDesktop ? <section className="panel ai-intro">
      <WifiOff size={28}/><h3>Open the Windows desktop app</h3>
      <p>AI chat is desktop-only in V0.7. Browser mode retains memory and evidence features; it does not expose a model API to browser pages.</p>
      <code>cd apps\exo-web; npm run desktop</code>
    </section> : <>
      <section className="panel ai-controls" aria-label="Local model connection">
        <div className="ai-controls-copy"><BrainCircuit size={20}/><div>
          <strong>Ollama · local inference</strong>
          <span>{catalog?.detail ?? 'Checking for local AI models…'}</span>
        </div></div>
        <div className="ai-model-actions"><label htmlFor="ai-model">Model</label>
          <select id="ai-model" value={model} disabled={!catalog?.models.length || busy} onChange={e => setModel(e.target.value)}>
            {!catalog?.models.length && <option value="">No model available</option>}
            {catalog?.models.map(name => <option key={name} value={name}>{name}</option>)}
          </select>
          <button type="button" className="ghost-button ai-small-button" onClick={() => void checkModels()} disabled={checking || busy} title="Check local Ollama"><RefreshCw size={15}/> {checking ? 'Checking…' : 'Refresh'}</button>
        </div>
        {!catalog?.models.length && <p className="ai-setup">Install and start Ollama, then run <code>ollama pull qwen2.5:3b</code> in PowerShell. Refresh this panel afterward. The model is downloaded once and runs on your own computer.</p>}
      </section>

      <section className="panel ai-chat-panel">
        <div className="ai-chat-top">
          <div><span className="eyebrow">SESSION ONLY</span><h3>Your conversation</h3></div>
          <button className="ghost-button ai-small-button" type="button" disabled={!messages.length || busy} onClick={() => { setMessages([]); setExpanded({}); setError('') }}><Trash2 size={14}/> Clear chat</button>
        </div>
        <div className="ai-messages" role="log" aria-label="Cognitive chat messages">
          {!messages.length && <div className="ai-empty">
            <span className="ai-empty-icon"><MessageCircle size={27}/></span>
            <h3>Ask about your work.</h3>
            <p>Try “What is the latest milestone of EXOCORTEX?” or “What do my saved memories say about this project?”</p>
            <div className="ai-chips">
              <button type="button" onClick={() => setQuestion('What do my saved memories say about EXOCORTEX?')}>Summarize my project <ArrowRight size={13}/></button>
              <button type="button" onClick={() => setQuestion('What are the next steps based on my saved milestones?')}>Plan next steps <ArrowRight size={13}/></button>
            </div>
          </div>}
          {messages.map(item => <article key={item.id} className={'ai-message ai-' + item.role}>
            <div className="ai-message-avatar">{item.role === 'user' ? 'YOU' : <BrainCircuit size={17}/>}</div>
            <div className="ai-message-body">
              <div className="ai-message-heading">{item.role === 'user' ? 'You' : 'EXOCORTEX · AI-generated'} {item.model && <span>{item.model}</span>}</div>
              <div className="ai-message-content">{item.content}</div>
              {item.role === 'assistant' && <div className="ai-citation-block">
                <button className="ai-source-toggle" type="button" aria-expanded={!!expanded[item.id]} onClick={() => setExpanded(old => ({ ...old, [item.id]: !old[item.id] }))}>
                  <Database size={14}/> Context supplied: {item.sources?.length ?? 0} record(s) <ChevronDown size={14}/>
                </button>
                {expanded[item.id] && <div className="ai-source-list">
                  {!item.sources?.length && <p>No relevant saved records found for this question. This answer was not grounded in personal memory.</p>}
                  {item.sources?.map(source => <div className="ai-source" key={source.id}>
                    <div><strong>[{source.label}] {source.kind}</strong><span>{source.status}</span></div>
                    <p>{source.text}</p>
                    <code title={source.id}>Record {source.id}</code>
                    {source.source_event_id && <code title={source.source_event_id}>Source event {source.source_event_id}</code>}
                  </div>)}
                  <p className="ai-source-note">These are records supplied to the model, not proof that every statement in its answer is correct. Stale or disputed claims may appear with their status.</p>
                </div>}
              </div>}
            </div>
          </article>)}
          {busy && <div className="ai-pending"><span className="pulse"/> Thinking locally… the first model response may take time.</div>}
          <div ref={end}/>
        </div>
        {error && <div className="banner error ai-error" role="alert">{error}</div>}
        <form onSubmit={send} className="ai-composer">
          <textarea aria-label="Ask EXOCORTEX" value={question} onChange={e => setQuestion(e.target.value)} placeholder={model ? 'Ask about a project, memory, or next step…' : 'Start Ollama and choose an installed model first…'} maxLength={2000} rows={3} disabled={!model || busy}/>
          <div className="ai-compose-bottom"><span>{question.length}/2000 · No tools or automatic memory writes</span>
            <button className="primary" type="submit" disabled={!question.trim() || !model || busy}><Send size={16}/> {busy ? 'Thinking…' : 'Send'}</button>
          </div>
        </form>
      </section>
      <div className="ai-privacy"><ShieldCheck size={17}/><p><strong>Local-first by design.</strong> Your question, recent chat turns and a bounded selection of saved memories/facts go to Ollama on <code>127.0.0.1:11434</code>. Chat lives only in this open window and is not saved as memory automatically. AI may still make mistakes; review evidence before acting.</p></div>
    </>}
  </div>
}
