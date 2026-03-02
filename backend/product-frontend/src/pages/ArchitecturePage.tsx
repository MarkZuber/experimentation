import { useMemo } from 'react'

// plantuml-encoder is a CommonJS module; import as namespace
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import plantumlEncoder from 'plantuml-encoder'

const DIAGRAM = `
@startuml
!theme plain
skinparam backgroundColor #0f0f0f
skinparam ArrowColor #7c83fd
skinparam BorderColor #2a2a4a
skinparam FontColor #e0e0e0
skinparam ComponentBackgroundColor #1a1a2e
skinparam NodeBackgroundColor #1a1a2e

title ExperiProduct Architecture

node "k3s Cluster" {
  node "experiproduct pod" {
    component "nginx\\n(port 80)" as nginx
    component "product-backend\\n(port 8080)" as backend
    nginx --> backend : proxy /api/*
  }

  node "postgresql pod" {
    database "PostgreSQL\\n(port 5432)" as pg
    component "db-service\\n(gRPC 50051)" as dbs
    dbs --> pg : sqlx
  }

  node "redis pod" {
    database "Redis\\n(port 6379)" as redis
    component "cache-service\\n(gRPC 50052)" as cs
    cs --> redis : redis-rs
  }

  node "loki pod" {
    component "Loki\\n(port 3100)" as loki
  }

  node "grafana pod" {
    component "Grafana\\n(NodePort 3000)" as grafana
    grafana --> loki : datasource
  }

  backend --> dbs : gRPC items CRUD
  backend --> cs : gRPC cache ops
  backend --> loki : tracing-loki push
  dbs --> loki : tracing-loki push
  cs --> loki : tracing-loki push
}

actor "User" as user
user --> nginx : HTTP port 80

cloud "Google OAuth" as google
backend --> google : tokeninfo verify
@enduml
`

export default function ArchitecturePage() {
  const encoded = useMemo(() => plantumlEncoder.encode(DIAGRAM), [])
  const svgUrl = `https://www.plantuml.com/plantuml/svg/${encoded}`

  return (
    <div style={{ maxWidth: '900px', margin: '0 auto' }}>
      <h2 style={{ marginBottom: '1rem', color: '#e0e0e0' }}>Architecture</h2>
      <p style={{ color: '#a0a0b0', marginBottom: '1.5rem', fontSize: '0.9rem' }}>
        System architecture diagram showing all components and their communication paths.
      </p>
      <div style={{ background: '#1a1a2e', border: '1px solid #2a2a4a', borderRadius: '8px', padding: '1rem', textAlign: 'center' }}>
        <img
          src={svgUrl}
          alt="Architecture diagram"
          style={{ maxWidth: '100%', borderRadius: '4px' }}
        />
      </div>
    </div>
  )
}
