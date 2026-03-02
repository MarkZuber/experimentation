import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { GoogleOAuthProvider } from '@react-oauth/google'
import App from './App'

const googleAuthEnabled = import.meta.env.VITE_GOOGLE_AUTH_ENABLED === 'true'
const googleClientId = import.meta.env.VITE_GOOGLE_CLIENT_ID || ''

const app = (
  <BrowserRouter>
    <App />
  </BrowserRouter>
)

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {googleAuthEnabled ? (
      <GoogleOAuthProvider clientId={googleClientId}>
        {app}
      </GoogleOAuthProvider>
    ) : (
      app
    )}
  </React.StrictMode>
)
