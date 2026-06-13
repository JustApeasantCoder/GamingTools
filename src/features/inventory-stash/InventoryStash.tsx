import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { emitTo, listen } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Boxes, ExternalLink, Grid3X3, Pipette, Play, RefreshCw, X } from 'lucide-react'
import type { AppProfile, InventoryStashRule } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'

interface InventoryStashProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
  onPickPixel: () => Promise<{ color: string; x: number; y: number }>
  onSamplePixel: (request: { x: number; y: number }) => Promise<{ color: string; x: number; y: number }>
  onTestRule: (rule: InventoryStashRule) => Promise<number>
}

export function InventoryStash({ profile, onProfileChange, onPickPixel, onSamplePixel, onTestRule }: InventoryStashProps) {
  const rule = normalizeRule(profile.inventoryStashRules?.[0])
  const [testState, setTestState] = useState<'idle' | 'testing' | 'ready' | 'error'>('idle')
  const [occupiedCount, setOccupiedCount] = useState<number>()
  const [overlayOpen, setOverlayOpen] = useState(false)
  const [overlayError, setOverlayError] = useState<string>()
  const ruleRef = useRef(rule)
  const previewSlots = useMemo(() => createSlots(rule.columns, rule.rows), [rule.columns, rule.rows])

  const updateRule = useCallback((nextRule: InventoryStashRule) => {
    onProfileChange({ ...profile, inventoryStashRules: [nextRule] })
  }, [onProfileChange, profile])

  useEffect(() => {
    ruleRef.current = rule
  }, [rule])

  useEffect(() => {
    let unlistenGrid: (() => void) | undefined
    let unlistenReady: (() => void) | undefined
    let unlistenClosed: (() => void) | undefined

    void listen<InventoryStashRule['grid']>('inventory-overlay-grid-change', (event) => {
      updateRule({ ...ruleRef.current, grid: event.payload })
    }).then((dispose) => {
      unlistenGrid = dispose
    })

    void listen('inventory-overlay-ready', () => {
      setOverlayOpen(true)
      void emitTo('inventory-overlay', 'inventory-overlay-config', ruleRef.current)
    }).then((dispose) => {
      unlistenReady = dispose
    })

    void listen('inventory-overlay-closed', () => {
      setOverlayOpen(false)
    }).then((dispose) => {
      unlistenClosed = dispose
    })

    return () => {
      unlistenGrid?.()
      unlistenReady?.()
      unlistenClosed?.()
    }
  }, [updateRule])

  const toggleIgnoredSlot = (slotId: string) => {
    const isIgnored = rule.ignoredSlots.includes(slotId)
    const isWaystone = rule.waystoneSlots.includes(slotId)
    if (!isIgnored && !isWaystone) {
      updateRule({ ...rule, ignoredSlots: [...rule.ignoredSlots, slotId] })
      return
    }
    if (isIgnored) {
      updateRule({
        ...rule,
        ignoredSlots: rule.ignoredSlots.filter((item) => item !== slotId),
        waystoneSlots: [...rule.waystoneSlots, slotId],
      })
      return
    }
    updateRule({ ...rule, waystoneSlots: rule.waystoneSlots.filter((item) => item !== slotId) })
  }

  const pickEmptyColor = async () => {
    const result = await onPickPixel()
    updateRule({ ...rule, emptyColor: result.color })
  }

  const pickWaystoneColor = async () => {
    const result = await onPickPixel()
    updateRule({ ...rule, waystoneColor: result.color })
  }

  const sampleTopLeft = async () => {
    const result = await onSamplePixel({ x: rule.grid.x, y: rule.grid.y })
    updateRule({ ...rule, emptyColor: result.color })
  }

  const testRule = async () => {
    setTestState('testing')
    try {
      const count = await onTestRule(rule)
      setOccupiedCount(count)
      setTestState('ready')
    } catch {
      setTestState('error')
    }
  }

  const openOverlay = async () => {
    setOverlayError(undefined)
    if (!hasTauriRuntime()) {
      setOverlayError('Screen overlay is available in the desktop app.')
      return
    }

    const existing = await WebviewWindow.getByLabel('inventory-overlay')
    await existing?.close().catch(() => undefined)

    const overlay = new WebviewWindow('inventory-overlay', {
      url: overlayUrl(rule),
      title: 'Inventory Grid Overlay',
      ...physicalGridToLogicalWindow(rule.grid),
      transparent: true,
      decorations: false,
      alwaysOnTop: true,
      skipTaskbar: true,
      resizable: true,
      focusable: true,
      backgroundColor: [0, 0, 0, 0],
    })
    overlay.once('tauri://created', () => {
      setOverlayOpen(true)
      void emitTo('inventory-overlay', 'inventory-overlay-config', ruleRef.current)
    })
    overlay.once('tauri://error', (event) => {
      setOverlayError(String(event.payload))
      setOverlayOpen(false)
    })
  }

  const closeOverlay = async () => {
    const existing = await WebviewWindow.getByLabel('inventory-overlay')
    await existing?.close()
    setOverlayOpen(false)
  }

  return (
    <div className="feature-surface inventory-stash">
      <section className="macro-summary">
        <div>
          <h2>{rule.name}</h2>
          <p>
            <span>{rule.enabled ? 'Included in automation' : 'Not included in automation'}</span>
            <span>{rule.triggerKey}</span>
            <span>{rule.columns} x {rule.rows}</span>
            <span>{rule.ignoredSlots.length + rule.waystoneSlots.length} ignored</span>
          </p>
        </div>
        <div className="toolbar-group">
          <span className={`test-status ${testState === 'error' ? 'error' : testState === 'ready' ? 'matching' : ''}`}>{testLabel(testState, occupiedCount)}</span>
          <Button icon={Play} onClick={testRule} disabled={testState === 'testing'}>{testState === 'testing' ? 'Checking...' : 'Test slots'}</Button>
        </div>
      </section>

      <section className="inventory-layout">
        <section className="workflow-section">
          <header><span>1</span><div><h3>Stash shortcut</h3><p>Runs a foreground-only Ctrl + left click pass over occupied inventory slots.</p></div></header>
          <div className="inventory-control-grid">
            <label>Rule name<input value={rule.name} onChange={(event) => updateRule({ ...rule, name: event.target.value })} /></label>
            <label>Status<span className="editor-status-field"><span>{rule.enabled ? 'Included in automation' : 'Not included in automation'}</span><span className="switch-row compact"><input type="checkbox" checked={rule.enabled} onChange={(event) => updateRule({ ...rule, enabled: event.target.checked })} /></span></span></label>
            <label>Shortcut<KeyCaptureButton value={rule.triggerKey} onChange={(triggerKey) => updateRule({ ...rule, triggerKey })} label="Change shortcut" /></label>
          </div>
        </section>

        <section className="workflow-section">
          <header><span>2</span><div><h3>Detection</h3><p>Slots matching the empty color are skipped; every other slot is treated as occupied.</p></div></header>
          <div className="inventory-control-grid">
            <label>Empty slot color<div className="color-input-row"><input type="color" value={rule.emptyColor} onChange={(event) => updateRule({ ...rule, emptyColor: event.target.value })} /><input value={rule.emptyColor} onChange={(event) => updateRule({ ...rule, emptyColor: event.target.value })} /></div></label>
            <label>Tolerance<input type="number" min={0} max={255} value={rule.tolerance} onChange={(event) => updateRule({ ...rule, tolerance: Number(event.target.value) })} /></label>
            <label>Ignore Waystone<span className="editor-status-field"><span>{rule.ignoreWaystone ? 'Enabled' : 'Disabled'}</span><span className="switch-row compact"><input type="checkbox" checked={rule.ignoreWaystone} onChange={(event) => updateRule({ ...rule, ignoreWaystone: event.target.checked })} /></span></span></label>
            <label>Waystone color<div className="color-input-row"><input type="color" value={rule.waystoneColor} onChange={(event) => updateRule({ ...rule, waystoneColor: event.target.value })} /><input value={rule.waystoneColor} onChange={(event) => updateRule({ ...rule, waystoneColor: event.target.value })} /></div></label>
            <div className="inventory-button-stack">
              <Button icon={Pipette} onClick={pickEmptyColor}>Pick empty color</Button>
              <Button icon={Pipette} onClick={pickWaystoneColor}>Pick Waystone</Button>
              <Button icon={RefreshCw} onClick={sampleTopLeft}>Sample grid corner</Button>
            </div>
          </div>
        </section>

        <section className="workflow-section">
          <header><span>3</span><div><h3>Grid overlay</h3><p>Open the screen overlay, match it to the real inventory, then fine tune with exact numbers.</p></div></header>
          <div className="inventory-overlay-row">
            <div className="inventory-overlay-launcher">
              <Grid3X3 size={32} />
              <strong>{overlayOpen ? 'Screen overlay is open' : 'Screen overlay is closed'}</strong>
              <span>Use the floating overlay on top of the game inventory for real alignment.</span>
              <div className="inventory-overlay-actions">
                <Button icon={ExternalLink} variant="primary" onClick={openOverlay}>{overlayOpen ? 'Refresh overlay' : 'Open overlay'}</Button>
                <Button icon={X} onClick={closeOverlay} disabled={!overlayOpen}>Close overlay</Button>
              </div>
              {overlayError ? <div className="notice notice-error">{overlayError}</div> : null}
            </div>
            <div className="inventory-grid-fields">
              <label>X<input type="number" value={rule.grid.x} onChange={(event) => updateRule({ ...rule, grid: { ...rule.grid, x: Number(event.target.value) } })} /></label>
              <label>Y<input type="number" value={rule.grid.y} onChange={(event) => updateRule({ ...rule, grid: { ...rule.grid, y: Number(event.target.value) } })} /></label>
              <label>Width<input type="number" min={120} value={rule.grid.width} onChange={(event) => updateRule({ ...rule, grid: { ...rule.grid, width: Number(event.target.value) } })} /></label>
              <label>Height<input type="number" min={80} value={rule.grid.height} onChange={(event) => updateRule({ ...rule, grid: { ...rule.grid, height: Number(event.target.value) } })} /></label>
            </div>
          </div>
        </section>
      </section>

      <section className="workflow-section">
        <header><span><Grid3X3 size={16} /></span><div><h3>Ignored slots</h3><p>Click once for always ignore; click twice for Waystone-only ignore.</p></div></header>
        <div className="inventory-ignore-grid" style={{ gridTemplateColumns: `repeat(${rule.columns}, 1fr)` }}>
          {previewSlots.map((slot) => {
            const ignored = rule.ignoredSlots.includes(slot)
            const waystone = rule.waystoneSlots.includes(slot)
            const [column, row] = slot.split(':').map(Number)
            return <button key={slot} className={waystone ? 'waystone' : ignored ? 'ignored' : ''} onClick={() => toggleIgnoredSlot(slot)}>{column + 1},{row + 1}</button>
          })}
        </div>
      </section>

      <section className="workflow-section">
        <header><span><Boxes size={16} /></span><div><h3>Action timing</h3><p>One randomized range controls click waits, slot checks, and the pause after each sent slot.</p></div></header>
        <div className="inventory-control-grid compact">
          <label>Enabled<span className="editor-status-field"><span>{rule.humanization.enabled ? 'Use timing range' : 'No added delay'}</span><span className="switch-row compact"><input type="checkbox" checked={rule.humanization.enabled} onChange={(event) => updateRule({ ...rule, humanization: { ...rule.humanization, enabled: event.target.checked } })} /></span></span></label>
          <label>Minimum ms<input type="number" min={0} value={rule.humanization.minMs} onChange={(event) => updateRule({ ...rule, humanization: { ...rule.humanization, minMs: Number(event.target.value) } })} /></label>
          <label>Maximum ms<input type="number" min={0} value={rule.humanization.maxMs} onChange={(event) => updateRule({ ...rule, humanization: { ...rule.humanization, maxMs: Number(event.target.value) } })} /></label>
        </div>
      </section>
    </div>
  )
}

function hasTauriRuntime() {
  return '__TAURI_INTERNALS__' in window
}

function overlayUrl(rule: InventoryStashRule) {
  const params = new URLSearchParams({
    view: 'inventory-overlay',
    x: String(rule.grid.x),
    y: String(rule.grid.y),
    width: String(rule.grid.width),
    height: String(rule.grid.height),
    columns: String(rule.columns),
    rows: String(rule.rows),
  })
  return `/?${params.toString()}`
}

function physicalGridToLogicalWindow(grid: InventoryStashRule['grid']) {
  const scale = window.devicePixelRatio || 1
  return {
    x: Math.round(grid.x / scale),
    y: Math.round(grid.y / scale),
    width: Math.round(grid.width / scale),
    height: Math.round(grid.height / scale),
  }
}

function normalizeRule(rule?: InventoryStashRule): InventoryStashRule {
  const sanitizedRule = rule ?? {}
  const defaultRule: InventoryStashRule = {
    id: crypto.randomUUID(),
    name: 'Inventory to stash',
    enabled: false,
    triggerKey: 'F6',
    columns: 12,
    rows: 5,
    grid: { x: 34, y: 37, width: 844, height: 352 },
    emptyColor: '#0f1110',
    ignoreWaystone: false,
    waystoneColor: '#7a52c8',
    tolerance: 18,
    ignoredSlots: [],
    waystoneSlots: [],
    humanization: { enabled: true, minMs: 120, maxMs: 240 },
  }
  return {
    ...defaultRule,
    ...sanitizedRule,
    grid: { ...defaultRule.grid, ...rule?.grid },
    ignoredSlots: rule?.ignoredSlots ?? defaultRule.ignoredSlots,
    waystoneSlots: rule?.waystoneSlots ?? defaultRule.waystoneSlots,
    humanization: { ...defaultRule.humanization, ...rule?.humanization },
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

function testLabel(state: 'idle' | 'testing' | 'ready' | 'error', count?: number) {
  if (state === 'testing') return 'Checking...'
  if (state === 'ready') return `${count ?? 0} occupied`
  if (state === 'error') return 'Target unavailable'
  return 'Not tested'
}
