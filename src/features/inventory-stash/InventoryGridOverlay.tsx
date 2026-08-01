import { useEffect, useMemo, useRef, useState } from 'react'
import type React from 'react'
import { emitTo, listen } from '@tauri-apps/api/event'
import { PhysicalPosition, PhysicalSize, getCurrentWindow } from '@tauri-apps/api/window'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { InventoryStashRule, MapCrafterRule, StashInventoryRule, TabletScannerRule } from '../../shared/types/profile'
import '../../App.css'

type DragMode = 'move' | 'resize' | undefined
type OverlayView = 'inventory-overlay' | 'stash-inventory-overlay' | 'tablet-scanner-overlay' | 'map-crafter-overlay'
type OverlayRule = InventoryStashRule | StashInventoryRule | TabletScannerRule | MapCrafterRule

export function InventoryGridOverlay() {
  const view = overlayViewFromUrl()
  const events = useMemo(() => overlayEvents(view), [view])
  const [rule, setRule] = useState<OverlayRule | undefined>(() => ruleFromUrl(view) ?? (hasTauriRuntime() ? undefined : previewRule(view)))
  const ruleRef = useRef<OverlayRule | undefined>(rule)
  const slots = useMemo(() => createSlots(rule?.columns ?? 12, rule?.rows ?? 5), [rule?.columns, rule?.rows])

  useEffect(() => {
    ruleRef.current = rule
  }, [rule])

  useEffect(() => {
    if (!hasTauriRuntime()) return
    const appWindow = getCurrentWindow()
    const webviewWindow = getCurrentWebviewWindow()
    let cancelled = false
    let publishTimer: ReturnType<typeof setTimeout> | undefined
    void webviewWindow.setBackgroundColor([0, 0, 0, 0])
    void appWindow.setAlwaysOnTop(true)
    void appWindow.setDecorations(false)
    if (ruleRef.current) {
      void syncWindowToPhysicalGrid(appWindow, ruleRef.current.grid).then(() => publishActualGrid(appWindow, ruleRef))
    }

    let unlistenConfig: (() => void) | undefined
    let unlistenMoved: (() => void) | undefined
    let unlistenResized: (() => void) | undefined
    const scheduleGridPublish = () => {
      if (publishTimer) clearTimeout(publishTimer)
      publishTimer = setTimeout(() => {
        publishTimer = undefined
        void publishActualGrid(appWindow, ruleRef)
      }, 150)
    }

    void listen<OverlayRule>(events.config, (event) => {
      ruleRef.current = event.payload
      setRule(event.payload)
      void syncWindowToPhysicalGrid(appWindow, event.payload.grid).then(() => publishActualGrid(appWindow, ruleRef))
    }).then((dispose) => {
      if (cancelled) dispose()
      else unlistenConfig = dispose
    })
    void appWindow.onMoved(scheduleGridPublish).then((dispose) => {
      if (cancelled) dispose()
      else unlistenMoved = dispose
    })
    void appWindow.onResized(scheduleGridPublish).then((dispose) => {
      if (cancelled) dispose()
      else unlistenResized = dispose
    })
    void emitTo('main', events.ready)

    return () => {
      cancelled = true
      if (publishTimer) clearTimeout(publishTimer)
      unlistenConfig?.()
      unlistenMoved?.()
      unlistenResized?.()
    }
  }, [events.config, events.ready])

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
        className={view === 'tablet-scanner-overlay' || view === 'map-crafter-overlay' ? 'inventory-screen-grid tablet-screen-grid' : 'inventory-screen-grid'}
        style={{
          gridTemplateColumns: `repeat(${rule.columns}, 1fr)`,
          gridTemplateRows: `repeat(${rule.rows}, 1fr)`,
        }}
        onPointerDown={(event) => beginDrag(event, 'move')}
      >
        <div className="inventory-screen-drag-label">{view === 'inventory-overlay' ? 'Drag inventory grid' : 'Drag stash grid'}</div>
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

function previewRule(view: OverlayView): OverlayRule {
  const grid = view === 'inventory-overlay'
    ? { x: 34, y: 37, width: 844, height: 352 }
    : { x: 18, y: 126, width: 632, height: 632 }
  const base = {
    id: 'inventory-stash-preview',
    columns: 12,
    rows: view === 'inventory-overlay' ? 5 : 12,
    grid,
  }
  if (view === 'tablet-scanner-overlay') {
    return {
      ...base,
      id: 'tablet-scanner-preview',
      name: 'Tablet stash scanner',
      triggerKey: 'F9',
      targetExecutable: '',
      scanDelayMs: 90,
      emptyTolerance: 8,
      craft: {
        transmutation: { x: 0, y: 0 },
        augmentation: { x: 0, y: 0 },
        regal: { x: 0, y: 0 },
        exalted: { x: 0, y: 0 },
        alchemy: { x: 0, y: 0 },
        tabSwitchDelayMs: 120,
        craftDelayMs: 90,
      },
      valueRules: [],
    }
  }
  if (view === 'map-crafter-overlay') {
    return {
      ...base,
      id: 'map-crafter-preview',
      name: 'Map crafter',
      rows: 6,
      grid: { x: 18, y: 126, width: 632, height: 316 },
      targetExecutable: '',
      scanDelayMs: 90,
      emptyTolerance: 8,
      craft: {
        alchemy: { x: 0, y: 0 },
        exalted: { x: 0, y: 0 },
        scouring: { x: 0, y: 0 },
        craftDelayMs: 90,
      },
    }
  }
  if (view === 'stash-inventory-overlay') {
    return {
      ...base,
      id: 'stash-inventory-preview',
      name: 'Stash to inventory',
      enabled: false,
      triggerKey: 'F10',
      emptyColor: '#17130f',
      tolerance: 18,
      humanization: { enabled: true, minMs: 120, maxMs: 240 },
    }
  }
  return {
    ...base,
    name: 'Inventory to stash',
    enabled: false,
    triggerKey: 'F6',
    captureBaselineKey: 'F8',
    detectionMode: 'emptyColor',
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

function overlayViewFromUrl(): OverlayView {
  const view = new URLSearchParams(window.location.search).get('view')
  if (view === 'map-crafter-overlay') return 'map-crafter-overlay'
  if (view === 'tablet-scanner-overlay') return 'tablet-scanner-overlay'
  if (view === 'stash-inventory-overlay') return 'stash-inventory-overlay'
  return 'inventory-overlay'
}

function ruleFromUrl(view: OverlayView): OverlayRule | undefined {
  const params = new URLSearchParams(window.location.search)
  if (params.get('view') !== view) return undefined
  const x = Number(params.get('x'))
  const y = Number(params.get('y'))
  const width = Number(params.get('width'))
  const height = Number(params.get('height'))
  const columns = Number(params.get('columns') ?? 12)
  const rows = Number(params.get('rows') ?? 5)
  if (![x, y, width, height, columns, rows].every(Number.isFinite)) return undefined
  return {
    ...previewRule(view),
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
  await emitTo('main', overlayEvents(overlayViewFromUrl()).closed)
  await getCurrentWindow().close()
}

function overlayEvents(view: OverlayView) {
  if (view === 'map-crafter-overlay') {
    return {
      config: 'map-crafter-overlay-config',
      ready: 'map-crafter-overlay-ready',
      closed: 'map-crafter-overlay-closed',
      gridChange: 'map-crafter-overlay-grid-change',
    }
  }
  if (view === 'tablet-scanner-overlay') {
    return {
      config: 'tablet-scanner-overlay-config',
      ready: 'tablet-scanner-overlay-ready',
      closed: 'tablet-scanner-overlay-closed',
      gridChange: 'tablet-scanner-overlay-grid-change',
    }
  }
  if (view === 'stash-inventory-overlay') {
    return {
      config: 'stash-inventory-overlay-config',
      ready: 'stash-inventory-overlay-ready',
      closed: 'stash-inventory-overlay-closed',
      gridChange: 'stash-inventory-overlay-grid-change',
    }
  }
  return {
    config: 'inventory-overlay-config',
    ready: 'inventory-overlay-ready',
    closed: 'inventory-overlay-closed',
    gridChange: 'inventory-overlay-grid-change',
  }
}

async function syncWindowToPhysicalGrid(appWindow: ReturnType<typeof getCurrentWindow>, grid: OverlayRule['grid']) {
  await appWindow.setPosition(new PhysicalPosition(grid.x, grid.y))
  await appWindow.setSize(new PhysicalSize(grid.width, grid.height))
}

async function publishActualGrid(
  appWindow: ReturnType<typeof getCurrentWindow>,
  ruleRef: React.MutableRefObject<OverlayRule | undefined>,
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
  if (gridsEqual(currentRule.grid, grid)) return
  ruleRef.current = { ...currentRule, grid }
  await emitTo('main', overlayEvents(overlayViewFromUrl()).gridChange, grid)
}

function gridsEqual(left: OverlayRule['grid'], right: OverlayRule['grid']) {
  return left.x === right.x
    && left.y === right.y
    && left.width === right.width
    && left.height === right.height
}
