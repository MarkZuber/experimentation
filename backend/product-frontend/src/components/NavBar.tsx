import { Link, useNavigate } from 'react-router-dom'

const navStyle: React.CSSProperties = {
  background: '#1a1a2e',
  padding: '0.75rem 2rem',
  display: 'flex',
  alignItems: 'center',
  gap: '1.5rem',
  borderBottom: '1px solid #2a2a4a',
}

const linkStyle: React.CSSProperties = {
  color: '#7c83fd',
  textDecoration: 'none',
  fontWeight: 500,
  fontSize: '0.95rem',
}

const brandStyle: React.CSSProperties = {
  color: '#e0e0e0',
  fontWeight: 700,
  fontSize: '1.1rem',
  marginRight: '1rem',
}

const btnStyle: React.CSSProperties = {
  marginLeft: 'auto',
  background: '#e53e3e',
  color: '#fff',
  border: 'none',
  borderRadius: '4px',
  padding: '0.4rem 1rem',
  cursor: 'pointer',
  fontSize: '0.9rem',
}

export default function NavBar() {
  const navigate = useNavigate()
  const token = localStorage.getItem('jwt')

  const signOut = () => {
    localStorage.removeItem('jwt')
    navigate('/')
  }

  return (
    <nav style={navStyle}>
      <span style={brandStyle}>ExperiProduct</span>
      <Link to="/" style={linkStyle}>Home</Link>
      <Link to="/items" style={linkStyle}>Items</Link>
      <Link to="/redis" style={linkStyle}>Redis</Link>
      <Link to="/telemetry" style={linkStyle}>Telemetry</Link>
      <Link to="/architecture" style={linkStyle}>Architecture</Link>
      {token && (
        <button onClick={signOut} style={btnStyle}>Sign Out</button>
      )}
    </nav>
  )
}
