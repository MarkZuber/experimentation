import { Routes, Route, Navigate } from 'react-router-dom'
import NavBar from './components/NavBar'
import WelcomePage from './pages/WelcomePage'
import ItemsPage from './pages/ItemsPage'
import RedisPage from './pages/RedisPage'
import TelemetryPage from './pages/TelemetryPage'
import ArchitecturePage from './pages/ArchitecturePage'

function AuthGuard({ children }: { children: React.ReactNode }) {
  const token = localStorage.getItem('jwt')
  if (!token) return <Navigate to="/" replace />
  return <>{children}</>
}

export default function App() {
  return (
    <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column' }}>
      <NavBar />
      <main style={{ flex: 1, padding: '2rem' }}>
        <Routes>
          <Route path="/" element={<WelcomePage />} />
          <Route path="/items" element={<AuthGuard><ItemsPage /></AuthGuard>} />
          <Route path="/redis" element={<AuthGuard><RedisPage /></AuthGuard>} />
          <Route path="/telemetry" element={<AuthGuard><TelemetryPage /></AuthGuard>} />
          <Route path="/architecture" element={<ArchitecturePage />} />
        </Routes>
      </main>
    </div>
  )
}
