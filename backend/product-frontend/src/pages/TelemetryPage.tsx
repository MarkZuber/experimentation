export default function TelemetryPage() {
  const grafanaUrl = `http://${window.location.hostname}:3000`

  return (
    <div style={{ height: 'calc(100vh - 120px)', display: 'flex', flexDirection: 'column' }}>
      <h2 style={{ marginBottom: '1rem', color: '#e0e0e0' }}>Telemetry</h2>
      <p style={{ color: '#a0a0b0', marginBottom: '1rem', fontSize: '0.9rem' }}>
        Grafana with Loki logs from all backend services.{' '}
        <a href={grafanaUrl} target="_blank" rel="noopener noreferrer" style={{ color: '#7c83fd' }}>
          Open in new tab
        </a>
      </p>
      <iframe
        src={grafanaUrl}
        style={{
          flex: 1,
          border: '1px solid #2a2a4a',
          borderRadius: '8px',
          background: '#fff',
        }}
        title="Grafana"
      />
    </div>
  )
}
