import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'

interface KVPair {
  key: string
  value: string
}

function authHeaders() {
  return { Authorization: `Bearer ${localStorage.getItem('jwt')}` }
}

export default function RedisPage() {
  const navigate = useNavigate()
  const [pairs, setPairs] = useState<KVPair[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const fetchPairs = async () => {
    setLoading(true)
    const resp = await fetch('/api/redis', { headers: authHeaders() })
    if (resp.status === 401) { navigate('/'); return }
    if (resp.ok) {
      setPairs(await resp.json())
      setError('')
    } else {
      const body = await resp.json().catch(() => ({}))
      setError(body.error || `Request failed (${resp.status})`)
    }
    setLoading(false)
  }

  useEffect(() => {
    fetchPairs()
    const interval = setInterval(fetchPairs, 5000)
    return () => clearInterval(interval)
  }, [])

  return (
    <div style={{ maxWidth: '700px', margin: '0 auto' }}>
      <h2 style={{ marginBottom: '0.5rem', color: '#e0e0e0' }}>Redis Keys</h2>
      <p style={{ color: '#a0a0b0', marginBottom: '1.5rem', fontSize: '0.9rem' }}>
        Auto-refreshes every 5 seconds. The backend writes a heartbeat key every 5s with a 180s TTL.
      </p>

      {error && <p style={{ color: '#fc8181', marginBottom: '1rem' }}>{error}</p>}
      {loading && pairs.length === 0 ? (
        <p style={{ color: '#a0a0b0' }}>Loading...</p>
      ) : (
        <div style={{ background: '#1a1a2e', border: '1px solid #2a2a4a', borderRadius: '8px', overflow: 'hidden' }}>
          {pairs.length === 0 ? (
            <p style={{ padding: '2rem', textAlign: 'center', color: '#666' }}>No keys in Redis</p>
          ) : (
            pairs.map((pair, i) => (
              <div key={i} style={{
                display: 'flex',
                gap: '1rem',
                padding: '0.75rem 1rem',
                borderBottom: '1px solid #2a2a4a',
                fontFamily: 'monospace',
                fontSize: '0.9rem',
              }}>
                <span style={{ color: '#7c83fd', minWidth: '200px', overflow: 'hidden', textOverflow: 'ellipsis' }}>{pair.key}</span>
                <span style={{ color: '#a0a0b0' }}>{pair.value}</span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  )
}
