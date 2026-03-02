import { useNavigate } from 'react-router-dom'
import { GoogleLogin } from '@react-oauth/google'

const googleAuthEnabled = import.meta.env.VITE_GOOGLE_AUTH_ENABLED === 'true'

const containerStyle: React.CSSProperties = {
  maxWidth: '480px',
  margin: '4rem auto',
  textAlign: 'center',
}

const headingStyle: React.CSSProperties = {
  fontSize: '2.5rem',
  fontWeight: 700,
  marginBottom: '0.5rem',
  color: '#e0e0e0',
}

const subStyle: React.CSSProperties = {
  fontSize: '1.1rem',
  color: '#a0a0b0',
  marginBottom: '2.5rem',
}

const cardStyle: React.CSSProperties = {
  background: '#1a1a2e',
  border: '1px solid #2a2a4a',
  borderRadius: '12px',
  padding: '2rem',
  display: 'inline-block',
}

const devBtnStyle: React.CSSProperties = {
  background: '#7c83fd',
  color: '#fff',
  border: 'none',
  borderRadius: '6px',
  padding: '0.7rem 2rem',
  cursor: 'pointer',
  fontSize: '1rem',
  fontWeight: 600,
}

export default function WelcomePage() {
  const navigate = useNavigate()
  const token = localStorage.getItem('jwt')

  if (token) {
    return (
      <div style={containerStyle}>
        <h1 style={headingStyle}>Welcome back!</h1>
        <p style={subStyle}>You are signed in.</p>
        <button
          style={{ background: '#7c83fd', color: '#fff', border: 'none', borderRadius: '6px', padding: '0.6rem 1.4rem', cursor: 'pointer', fontSize: '1rem' }}
          onClick={() => navigate('/items')}
        >
          Go to Items
        </button>
      </div>
    )
  }

  return (
    <div style={containerStyle}>
      <h1 style={headingStyle}>ExperiProduct</h1>
      <p style={subStyle}>A Rust + React app running on k3s</p>
      <div style={cardStyle}>
        {googleAuthEnabled ? (
          <>
            <p style={{ marginBottom: '1.5rem', color: '#a0a0b0' }}>Sign in with Google to continue</p>
            <GoogleLogin
              onSuccess={async (credentialResponse) => {
                const idToken = credentialResponse.credential
                if (!idToken) return
                const resp = await fetch('/api/auth/google', {
                  method: 'POST',
                  headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify({ id_token: idToken }),
                })
                if (resp.ok) {
                  const data = await resp.json()
                  localStorage.setItem('jwt', data.token)
                  navigate('/items')
                } else {
                  alert('Authentication failed')
                }
              }}
              onError={() => alert('Google login failed')}
              useOneTap
            />
          </>
        ) : (
          <>
            <p style={{ marginBottom: '1.5rem', color: '#a0a0b0' }}>Sign in to continue</p>
            <button
              style={devBtnStyle}
              onClick={async () => {
                const resp = await fetch('/api/auth/dev', { method: 'POST' })
                if (resp.ok) {
                  const data = await resp.json()
                  localStorage.setItem('jwt', data.token)
                  navigate('/items')
                } else {
                  alert('Sign in failed')
                }
              }}
            >
              Sign In
            </button>
          </>
        )}
      </div>
    </div>
  )
}
