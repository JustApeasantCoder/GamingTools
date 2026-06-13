import { useEffect, useMemo, useRef, useState } from 'react'
import type React from 'react'
import { emitTo, listen } from '@tauri-apps/api/event'
import { PhysicalPosition, PhysicalSize, getCurrentWindow } from '@tauri-apps/api/window'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { InventoryStashRule } from '../../shared/types/profile'
import '../../App.css'

type DragMode = 'move' | 'resize' | undefined

export function InventoryGridOverlay() {
  const [rule, setRule] = useState<InventoryStashRule | undefined>(() => ruleFromUrl() ?? (hasTauriRuntime() ? undefined : previewRule()))
  const ruleRef = useRef<InventoryStashRule | undefined>(rule)
  const slots = useMemo(() => createSlots(rule?.columns ?? 12, rule?.rows ?? 5), [rule?.columns, rule?.rows])

  useEffect(() => {
    ruleRef.current = rule
  }, [rule])

  useEffect(() => {
    if (!hasTauriRuntime()) return
    const appWindow = getCurrentWindow()
    const webviewWindow = getCurrentWebviewWindow()
    void webviewWindow.setBackgroundColor([0, 0, 0, 0])
    void appWindow.setAlwaysOnTop(true)
    void appWindow.setDecorations(false)
    if (ruleRef.current) {
      void syncWindowToPhysicalGrid(appWindow, ruleRef.current.grid).then(() => publishActualGrid(appWindow, ruleRef))
    }

    let unlistenConfig: (() => void) | undefined
    let unlistenMoved: (() => void) | undefined
    let unlistenResized: (() => void) | undefined
    void listen<InventoryStashRule>('inventory-overlay-config', (event) => {
      setRule(event.payload)
      void syncWindowToPhysicalGrid(appWindow, event.payload.grid).then(() => publishActualGrid(appWindow, ruleRef))
    }).then((dispose) => {
      unlistenConfig = dispose
    })
    void appWindow.onMoved(() => {
      void publishActualGrid(appWindow, ruleRef)
    }).then((dispose) => {
      unlistenMoved = dispose
    })
    void appWindow.onResized(() => {
      void publishActualGrid(appWindow, ruleRef)
    }).then((dispose) => {
      unlistenResized = dispose
    })
    void emitTo('main', 'inventory-overlay-ready')

    return () => {
      unlistenConfig?.()
      unlistenMoved?.()
      unlistenResized?.()
    }
  }, [])

  const beginDrag = (event: React.PointerEvent, mode: DragMode) => {
    if (!rule) return
    if (!hasTauriRuntime()) return
    event.preventDefault()
    const appWindow = getCurrentWindow()
    if (mode === 'resize') {
      void appWindow.startResizeDragging('SouthEast')
    } else {
      void appWindow.startDragging()
    }
  }

  if (!rule) {
    return <main className="inventory-screen-overlay loading">Waiting for grid...</main>
  }

  return (
    <main className="inventory-screen-overlay">
      <section
        className="inventory-screen-grid"
        style={{
          gridTemplateColumns: `repeat(${rule.columns}, 1fr)`,
          gridTemplateRows: `repeat(${rule.rows}, 1fr)`,
        }}
        onPointerDown={(event) => beginDrag(event, 'move')}
      >
        <div className="inventory-screen-drag-label">Drag grid</div>
        {slots.map((slot) => <span key={slot} />)}
        <button className="inventory-screen-close" aria-label="Close inventory grid overlay" onPointerDown={(event) => event.stopPropagation()} onClick={() => void closeOverlay()} />
        <button className="inventory-screen-resize" aria-label="Resize inventory grid" onPointerDown={(event) => { event.stopPropagation(); beginDrag(event, 'resize') }} />
      </section>
    </main>
  )
}

function hasTauriRuntime() {
  return '__TAURI_INTERNALS__' in window
}

function previewRule(): InventoryStashRule {
  return {
    id: 'inventory-stash-preview',
    name: 'Inventory to stash',
    enabled: false,
    triggerKey: 'F6',
    captureBaselineKey: 'F8',
    detectionMode: 'emptyColor',
    columns: 12,
    rows: 5,
    grid: { x: 34, y: 37, width: 844, height: 352 },
    emptyColor: '#0f1110',
    ignoreWaystone: false,
    waystoneColor: '#7a52c8',
    tolerance: 18,
    ignoredSlots: [],
    waystoneSlots: [],
    snapshotColors: [],
    humanization: { enabled: true, minMs: 120, maxMs: 240 },
  }
}

function createSlots(columns: number, rows: number) {
  const slots: string[] = []
  for (let row = 0; row < rows; row += 1) {
    for (let column = 0; column < columns; column += 1) {
      slots.push(`${column}:${row}`)
    }
  }
  return slots
}

function ruleFromUrl(): InventoryStashRule | undefined {
  const params = new URLSearchParams(window.location.search)
  if (params.get('view') !== 'inventory-overlay') return undefined
  const x = Number(params.get('x'))
  const y = Number(params.get('y'))
  const width = Number(params.get('width'))
  const height = Number(params.get('height'))
  const columns = Number(params.get('columns') ?? 12)
  const rows = Number(params.get('rows') ?? 5)
  if (![x, y, width, height, columns, rows].every(Number.isFinite)) return undefined
  return {
    ...previewRule(),
    columns,
    rows,
    grid: {
      x,
      y,
      width: Math.max(120, width),
      height: Math.max(80, height),
    },
  }
}

async function closeOverlay() {
  if (!hasTauriRuntime()) return
  await emitTo('main', 'inventory-overlay-closed')
  await getCurrentWindow().close()
}

async function syncWindowToPhysicalGrid(appWindow: ReturnType<typeof getCurrentWindow>, grid: InventoryStashRule['grid']) {
  await appWindow.setPosition(new PhysicalPosition(grid.x, grid.y))
  await appWindow.setSize(new PhysicalSize(grid.width, grid.height))
}

async function publishActualGrid(
  appWindow: ReturnType<typeof getCurrentWindow>,
  ruleRef: React.MutableRefObject<InventoryStashRule | undefined>,
) {
  const currentRule = ruleRef.current
  if (!currentRule) return
  const [position, size] = await Promise.all([
    appWindow.innerPosition(),
    appWindow.innerSize(),
  ])
  const grid = {
    x: Math.round(position.x),
    y: Math.round(position.y),
    width: Math.max(120, Math.round(size.width)),
    height: Math.max(80, Math.round(size.height)),
  }
  ruleRef.current = { ...currentRule, grid }
  await emitTo('main', 'inventory-overlay-grid-change', grid)
}
