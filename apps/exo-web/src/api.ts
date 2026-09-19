import { invoke, isTauri } from '@tauri-apps/api/core'

export type MemoryKind = 'Working' | 'Episodic' | 'Semantic' | 'Prospective'
export type EpistemicType = 'Observation' | 'UserConfirmedFact' | 'Inference' | 'Hypothesis' | 'Prediction'

export interface MemoryRecord {
  id: string
  text: string
  kind: MemoryKind
  epistemic_type: EpistemicType
  source_event_id: string
  created_at: string
  superseded_by: string | null
  provenance: Array<{ source_type: string; source_id: string }>
}
export interface StoredMemory { memory: MemoryRecord; topic_key: string | null }
export interface Status { memories: number; events: number; observations: number; facts: number; ai_connected: boolean; jarvis_connected: boolean }
export interface CognitiveEvent { id: string; kind: string; source: unknown; content: string; timestamp: string }
export interface MemoryConflict { other_id: string; topic_key: string; other_text: string }
export interface Observation { id: string; source_key: string; version: number; value: string; event_id: string; observed_at: string }
export interface WorkingFact { id: string; entity: string; attribute: string; value: string; kind: 'Observed' | 'Derived'; status: 'Observed' | 'Derived' | 'Stale' | 'Disputed'; evidence_ids: string[]; created_at: string; stale_at: string | null }

export class ApiError extends Error { constructor(message: string, public status: number) { super(message) } }

// A packaged desktop app invokes the same Rust cognitive store directly. It does not
// launch a localhost HTTP server or need two terminal windows. The browser build
// retains the original V0.4 HTTP API, including its dev-server proxy.
const desktop = isTauri()

async function native<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  try { return await invoke<T>(command, args) }
  catch (error) { throw new ApiError(String(error), 0) }
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  let response: Response
  try {
    response = await fetch(`/api${path}`, {
      ...init,
      headers: { 'x-exocortex-client': 'local-ui-v1', ...(init.body ? { 'content-type': 'application/json' } : {}), ...init.headers },
    })
  } catch {
    throw new ApiError('Cannot reach the Rust API. Start cargo run -p exo-api in another terminal, or open the desktop application.', 0)
  }
  const body: unknown = await response.json().catch(() => null)
  if (!response.ok) {
    const message = typeof body === 'object' && body !== null && 'error' in body && typeof body.error === 'string'
      ? body.error : `Request failed (${response.status})`
    throw new ApiError(message, response.status)
  }
  return body as T
}

export const api = {
  health: () => desktop ? native<{ ok: boolean }>('workspace_status').then(() => ({ ok: true })) : request<{ ok: boolean }>('/health'),
  status: () => desktop ? native<Status>('workspace_status') : request<Status>('/status'),
  memories: (q = '') => desktop ? native<StoredMemory[]>('list_memories', { query: q }) : request<StoredMemory[]>(`/memories?q=${encodeURIComponent(q)}`),
  remember: (text: string, kind: MemoryKind, topic: string) => desktop ? native<StoredMemory>('remember', { text, kind, topic: topic.trim() || null }) : request<StoredMemory>('/memories', { method: 'POST', body: JSON.stringify({ text, kind, topic: topic.trim() || null }) }),
  correct: (id: string, text: string) => desktop ? native<StoredMemory>('correct_memory', { id, text }) : request<StoredMemory>(`/memories/${id}`, { method: 'PATCH', body: JSON.stringify({ text }) }),
  forget: (id: string) => desktop ? native<{ redacted_revisions: number }>('forget_memory', { id }) : request<{ redacted_revisions: number }>(`/memories/${id}`, { method: 'DELETE' }),
  source: (id: string) => desktop ? native<CognitiveEvent>('memory_source', { id }) : request<CognitiveEvent>(`/sources/${id}`),
  conflicts: (id: string) => desktop ? native<MemoryConflict[]>('memory_conflicts', { id }) : request<MemoryConflict[]>(`/conflicts/${id}`),
  facts: (entity = '') => desktop ? native<WorkingFact[]>('list_facts', { entity }) : request<WorkingFact[]>(`/facts?entity=${encodeURIComponent(entity)}`),
  observe: (source_key: string, version: number, value: string) => desktop ? native<Observation>('record_observation', { sourceKey: source_key, version, value }) : request<Observation>('/observations', { method: 'POST', body: JSON.stringify({ source_key, version, value }) }),
  latest: (source_key: string) => desktop ? native<Observation>('latest_observation', { sourceKey: source_key }) : request<Observation>(`/observations/latest?source_key=${encodeURIComponent(source_key)}`),
  addFact: (entity: string, attribute: string, value: string, kind: 'Observed' | 'Derived', evidence_ids: string[]) => desktop ? native<WorkingFact>('add_working_fact', { entity, attribute, value, kind, evidenceIds: evidence_ids }) :
    request<WorkingFact>('/facts', { method: 'POST', body: JSON.stringify({ entity, attribute, value, kind, evidence_ids }) }),
}
