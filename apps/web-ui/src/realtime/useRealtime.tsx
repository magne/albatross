import { useQueryClient } from '@tanstack/react-query'
import { createContext, type ReactNode, useCallback, useContext, useEffect, useRef, useState } from 'react'
import { useApiKey } from '../state/ApiKeyContext'
import { handleEventDispatch } from './mapper'

interface RealtimeContextValue {
  status: 'disconnected' | 'connecting' | 'connected' | 'error'
  lastError: string | null
  eventsProcessed: number
  manualReconnect: () => void
}

const RealtimeContext = createContext<RealtimeContextValue | undefined>(undefined)

interface WsEventEnvelope {
  type: string
  channel?: string
  payload?: unknown
}

export const RealtimeProvider = ({ children }: { children: ReactNode }) => {
  const { apiKey, userId } = useApiKey()
  const queryClient = useQueryClient()
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectAttempts = useRef(0)
  const [status, setStatus] = useState<RealtimeContextValue['status']>('disconnected')
  const [lastError, setLastError] = useState<string | null>(null)
  const [eventsProcessed, setEventsProcessed] = useState(0)
  const manualTriggerRef = useRef(0)

  const connect = useCallback(() => {
    if (!apiKey) {
      if (wsRef.current) {
        wsRef.current.close()
        wsRef.current = null
      }
      setStatus('disconnected')
      return
    }
    if (
      wsRef.current &&
      (wsRef.current.readyState === WebSocket.OPEN || wsRef.current.readyState === WebSocket.CONNECTING)
    ) {
      return
    }
    const base = import.meta.env.VITE_API_BASE || ''
    const url = `${base.replace(/^http/, 'ws').replace(/\/$/, '')}/api/ws?api_key=${encodeURIComponent(apiKey)}`
    setStatus('connecting')
    const ws = new WebSocket(url)
    wsRef.current = ws

    ws.onopen = () => {
      setStatus('connected')
      setLastError(null)
      reconnectAttempts.current = 0
      // Baseline subscriptions are auto-managed server side; no action needed.
    }
    ws.onmessage = (m) => {
      try {
        const parsed: WsEventEnvelope = JSON.parse(m.data)
        if (parsed.type === 'heartbeat') {
          // Could track latency later
          return
        }
        if (parsed.type === 'event') {
          handleEventDispatch(parsed, queryClient, userId)
          setEventsProcessed((c) => c + 1)
        }
      } catch (_e) {
        // Ignore malformed frames
      }
    }
    ws.onerror = () => {
      setLastError('WebSocket error')
      setStatus('error')
    }
    ws.onclose = () => {
      setStatus('disconnected')
      // Reconnect logic (cap attempts)
      if (apiKey && reconnectAttempts.current < 10 && manualTriggerRef.current === 0) {
        reconnectAttempts.current += 1
        const delay = Math.min(5000, 1000 + (reconnectAttempts.current - 1) * 500)
        setTimeout(() => {
          connect()
        }, delay)
      }
      manualTriggerRef.current = 0
    }
  }, [apiKey, queryClient, userId])

  useEffect(() => {
    connect()
    return () => {
      wsRef.current?.close()
    }
  }, [connect])

  const manualReconnect = useCallback(() => {
    manualTriggerRef.current = 1
    reconnectAttempts.current = 0
    wsRef.current?.close()
    // connect will be re-invoked by close handler with manual flag consumed
    connect()
  }, [connect])

  const value: RealtimeContextValue = {
    status,
    lastError,
    eventsProcessed,
    manualReconnect
  }

  return <RealtimeContext.Provider value={value}>{children}</RealtimeContext.Provider>
}

export function useRealtime() {
  const ctx = useContext(RealtimeContext)
  if (!ctx) throw new Error('useRealtime must be used within RealtimeProvider')
  return ctx
}
