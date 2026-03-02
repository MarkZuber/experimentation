import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'

interface Item {
  id: string
  name: string
  description: string
  created_at: string
  updated_at: string
}

const tableStyle: React.CSSProperties = {
  width: '100%',
  borderCollapse: 'collapse',
  background: '#1a1a2e',
  borderRadius: '8px',
  overflow: 'hidden',
}

const thStyle: React.CSSProperties = {
  background: '#2a2a4a',
  color: '#a0a0c0',
  padding: '0.75rem 1rem',
  textAlign: 'left',
  fontSize: '0.85rem',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
}

const tdStyle: React.CSSProperties = {
  padding: '0.75rem 1rem',
  borderBottom: '1px solid #2a2a4a',
  color: '#e0e0e0',
}

const btnStyle = (color: string): React.CSSProperties => ({
  background: color,
  color: '#fff',
  border: 'none',
  borderRadius: '4px',
  padding: '0.3rem 0.75rem',
  cursor: 'pointer',
  fontSize: '0.85rem',
  marginRight: '0.4rem',
})

const inputStyle: React.CSSProperties = {
  background: '#0f0f1a',
  border: '1px solid #3a3a5a',
  borderRadius: '4px',
  color: '#e0e0e0',
  padding: '0.5rem 0.75rem',
  fontSize: '0.95rem',
  width: '100%',
}

function authHeaders() {
  return { Authorization: `Bearer ${localStorage.getItem('jwt')}`, 'Content-Type': 'application/json' }
}

export default function ItemsPage() {
  const navigate = useNavigate()
  const [items, setItems] = useState<Item[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [newName, setNewName] = useState('')
  const [newDesc, setNewDesc] = useState('')
  const [editId, setEditId] = useState<string | null>(null)
  const [editName, setEditName] = useState('')
  const [editDesc, setEditDesc] = useState('')

  const fetchItems = async () => {
    setLoading(true)
    const resp = await fetch('/api/items', { headers: authHeaders() })
    if (resp.status === 401) { navigate('/'); return }
    if (resp.ok) {
      setItems(await resp.json())
    } else {
      setError('Failed to load items')
    }
    setLoading(false)
  }

  useEffect(() => { fetchItems() }, [])

  const createItem = async () => {
    if (!newName.trim()) return
    await fetch('/api/items', {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ name: newName, description: newDesc }),
    })
    setNewName('')
    setNewDesc('')
    fetchItems()
  }

  const startEdit = (item: Item) => {
    setEditId(item.id)
    setEditName(item.name)
    setEditDesc(item.description)
  }

  const saveEdit = async () => {
    if (!editId) return
    await fetch(`/api/items/${editId}`, {
      method: 'PUT',
      headers: authHeaders(),
      body: JSON.stringify({ name: editName, description: editDesc }),
    })
    setEditId(null)
    fetchItems()
  }

  const deleteItem = async (id: string) => {
    if (!confirm('Delete this item?')) return
    await fetch(`/api/items/${id}`, { method: 'DELETE', headers: authHeaders() })
    fetchItems()
  }

  return (
    <div style={{ maxWidth: '900px', margin: '0 auto' }}>
      <h2 style={{ marginBottom: '1.5rem', color: '#e0e0e0' }}>Items</h2>

      <div style={{ background: '#1a1a2e', border: '1px solid #2a2a4a', borderRadius: '8px', padding: '1.25rem', marginBottom: '1.5rem' }}>
        <h3 style={{ marginBottom: '1rem', color: '#a0a0c0', fontSize: '0.95rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Add Item</h3>
        <div style={{ display: 'flex', gap: '0.75rem' }}>
          <input style={inputStyle} placeholder="Name" value={newName} onChange={e => setNewName(e.target.value)} />
          <input style={inputStyle} placeholder="Description" value={newDesc} onChange={e => setNewDesc(e.target.value)} />
          <button style={{ ...btnStyle('#7c83fd'), whiteSpace: 'nowrap' }} onClick={createItem}>Add</button>
        </div>
      </div>

      {error && <p style={{ color: '#fc8181', marginBottom: '1rem' }}>{error}</p>}
      {loading ? (
        <p style={{ color: '#a0a0b0' }}>Loading...</p>
      ) : (
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Name</th>
              <th style={thStyle}>Description</th>
              <th style={thStyle}>Created</th>
              <th style={thStyle}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {items.length === 0 && (
              <tr><td colSpan={4} style={{ ...tdStyle, color: '#666', textAlign: 'center', padding: '2rem' }}>No items yet</td></tr>
            )}
            {items.map(item => (
              editId === item.id ? (
                <tr key={item.id}>
                  <td style={tdStyle}><input style={inputStyle} value={editName} onChange={e => setEditName(e.target.value)} /></td>
                  <td style={tdStyle}><input style={inputStyle} value={editDesc} onChange={e => setEditDesc(e.target.value)} /></td>
                  <td style={tdStyle}>{item.created_at.slice(0, 10)}</td>
                  <td style={tdStyle}>
                    <button style={btnStyle('#48bb78')} onClick={saveEdit}>Save</button>
                    <button style={btnStyle('#718096')} onClick={() => setEditId(null)}>Cancel</button>
                  </td>
                </tr>
              ) : (
                <tr key={item.id}>
                  <td style={tdStyle}>{item.name}</td>
                  <td style={tdStyle}>{item.description}</td>
                  <td style={tdStyle}>{item.created_at.slice(0, 10)}</td>
                  <td style={tdStyle}>
                    <button style={btnStyle('#7c83fd')} onClick={() => startEdit(item)}>Edit</button>
                    <button style={btnStyle('#e53e3e')} onClick={() => deleteItem(item.id)}>Delete</button>
                  </td>
                </tr>
              )
            ))}
          </tbody>
        </table>
      )}
    </div>
  )
}
