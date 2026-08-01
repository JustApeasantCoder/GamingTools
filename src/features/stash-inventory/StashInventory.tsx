import { useCallback, useEffect, useRef, useState } from 'react'
import { emitTo, listen } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { ExternalLink, Grid3X3, PackageOpen, Pipette, Play, RefreshCw, X } from 'lucide-react'
import type { AppProfile, StashInventoryRule } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'

interface StashInventoryProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
  onPickPixel: () => Promise<{ color: string; x: number; y: number }>
  onSamplePixel: (request: { x: number; y: number }) => Promise<{ color: string; x: number; y: number }>
  onTestRule: (rule: StashInventoryRule) => Promise<number>
}

export function StashInventory({ profile, onProfileChange, onPickPixel, onSamplePixel, onTestRule }: StashInventoryProps) {
  const rule = normalizeRule(profile.stashInventoryRules?.[0])
  const [testState, setTestState] = useState<'idle' | 'testing' | 'ready' | 'error'>('idle')
  const [occupiedCount, setOccupiedCount] = useState<number>()
  const [overlayOpen, setOverlayOpen] = useState(false)
  const [overlayError, setOverlayError] = useState<string>()
  const ruleRef = useRef(rule)

  const updateRule = useCallback((nextRule: StashInventoryRule) => {
    onProfileChange({ ...profile, stashInventoryRules: [nextRule] })
  }, [onProfileChange, profile])
  const updateRuleRef = useRef(updateRule)

  useEffect(() => {
    ruleRef.current = rule
    updateRuleRef.current = updateRule
  }, [rule, updateRule])

  useEffect(() => {
    let cancelled = false
    let unlistenGrid: (() => void) | undefined
    let unlistenReady: (() => void) | undefined
    let unlistenClosed: (() => void) | undefined

    void listen<StashInventoryRule['grid']>('stash-inventory-overlay-grid-change', (event) => {
      updateRuleRef.current({ ...ruleRef.current, grid: event.payload })
    }).then((dispose) => {
      if (cancelled) dispose()
      else unlistenGrid = dispose
    })

    void listen('stash-inventory-overlay-ready', () => {
      setOverlayOpen(true)
      void emitTo('stash-inventory-overlay', 'stash-inventory-overlay-config', ruleRef.current)
    }).then((dispose) => {
      if (cancelled) dispose()
      else unlistenReady = dispose
    })

    void listen('stash-inventory-overlay-closed', () => {
      setOverlayOpen(false)
    }).then((dispose) => {
      if (cancelled) dispose()
      else unlistenClosed = dispose
    })

    return () => {
      cancelled = true
      unlistenGrid?.()
      unlistenReady?.()
      unlistenClosed?.()
    }
  }, [])

  const pickEmptyColor = async () => {
    const result = await onPickPixel()
    updateRule({ ...rule, emptyColor: result.color })
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

    const existing = await WebviewWindow.getByLabel('stash-inventory-overlay')
    await existing?.close().catch(() => undefined)

    const overlay = new WebviewWindow('stash-inventory-overlay', {
      url: overlayUrl(rule),
      title: 'Stash Grid Overlay',
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
      void emitTo('stash-inventory-overlay', 'stash-inventory-overlay-config', ruleRef.current)
    })
    overlay.once('tauri://error', (event) => {
      setOverlayError(String(event.payload))
      setOverlayOpen(false)
    })
  }

  const closeOverlay = async () => {
    const existing = await WebviewWindow.getByLabel('stash-inventory-overlay')
    await existing?.close()
    setOverlayOpen(false)
  }

  return (
    <div className="feature-surface inventory-stash stash-inventory">
      <section className="macro-summary">
        <div>
          <h2>{rule.name}</h2>
          <p>
            <span>{rule.enabled ? 'Included in automation' : 'Not included in automation'}</span>
            <span>{rule.triggerKey}</span>
            <span>{rule.columns} x {rule.rows}</span>
            <span>No ignored slots</span>
          </p>
        </div>
        <div className="toolbar-group">
          <span className={`test-status ${testState === 'error' ? 'error' : testState === 'ready' ? 'matching' : ''}`}>{testLabel(testState, occupiedCount)}</span>
          <Button icon={Play} onClick={testRule} disabled={testState === 'testing'}>{testState === 'testing' ? 'Checking...' : 'Test slots'}</Button>
        </div>
      </section>

      <section className="inventory-layout">
        <section className="workflow-section">
          <header><span>1</span><div><h3>Inventory shortcut</h3><p>Runs a foreground-only Ctrl + left click pass over occupied stash slots.</p></div></header>
          <div className="inventory-control-grid inventory-rule-grid">
            <label>Rule name<input value={rule.name} onChange={(event) => updateRule({ ...rule, name: event.target.value })} /></label>
            <label>Status<span className="editor-status-field"><span>{rule.enabled ? 'Included in automation' : 'Not included in automation'}</span><span className="switch-row compact"><input type="checkbox" checked={rule.enabled} onChange={(event) => updateRule({ ...rule, enabled: event.target.checked })} /></span></span></label>
            <label>Shortcut<KeyCaptureButton value={rule.triggerKey} onChange={(triggerKey) => updateRule({ ...rule, triggerKey })} label="Change shortcut" /></label>
          </div>
        </section>

        <section className="workflow-section">
          <header><span>2</span><div><h3>Detection</h3><p>Slots matching the empty stash color are skipped; every other slot is moved. This direction intentionally has no ignored-slot configuration.</p></div></header>
          <div className="inventory-control-grid">
            <label>Empty slot color<div className="color-input-row"><input type="color" value={rule.emptyColor} onChange={(event) => updateRule({ ...rule, emptyColor: event.target.value })} /><input value={rule.emptyColor} onChange={(event) => updateRule({ ...rule, emptyColor: event.target.value })} /></div></label>
            <label>Tolerance<input type="number" min={0} max={255} value={rule.tolerance} onChange={(event) => updateRule({ ...rule, tolerance: Number(event.target.value) })} /></label>
            <div className="inventory-button-stack inventory-detection-actions">
              <Button icon={Pipette} onClick={pickEmptyColor}>Pick empty color</Button>
              <Button icon={RefreshCw} onClick={sampleTopLeft}>Sample grid corner</Button>
            </div>
          </div>
        </section>

        <section className="workflow-section">
          <header><span>3</span><div><h3>Stash grid overlay</h3><p>Open the screen overlay, match it to the stash tab, then fine tune the grid and slot count.</p></div></header>
          <div className="inventory-overlay-row">
            <div className="inventory-overlay-launcher">
              <Grid3X3 size={32} />
              <strong>{overlayOpen ? 'Stash overlay is open' : 'Stash overlay is closed'}</strong>
              <span>Use the floating overlay on top of the game stash for real alignment.</span>
              <div className="inventory-overlay-actions">
                <Button icon={ExternalLink} variant="primary" onClick={openOverlay}>{overlayOpen ? 'Refresh overlay' : 'Open overlay'}</Button>
                <Button icon={X} onClick={closeOverlay} disabled={!overlayOpen}>Close overlay</Button>
              </div>
              {overlayError ? <div className="notice notice-error">{overlayError}</div> : null}
            </div>
            <div className="inventory-grid-fields">
              <label>Columns<input type="number" min={1} max={64} value={rule.columns} onChange={(event) => updateRule({ ...rule, columns: gridCount(event.target.value) })} /></label>
              <label>Rows<input type="number" min={1} max={64} value={rule.rows} onChange={(event) => updateRule({ ...rule, rows: gridCount(event.target.value) })} /></label>
              <label>X<input type="number" value={rule.grid.x} onChange={(event) => updateRule({ ...rule, grid: { ...rule.grid, x: Number(event.target.value) } })} /></label>
              <label>Y<input type="number" value={rule.grid.y} onChange={(event) => updateRule({ ...rule, grid: { ...rule.grid, y: Number(event.target.value) } })} /></label>
              <label>Width<input type="number" min={120} value={rule.grid.width} onChange={(event) => updateRule({ ...rule, grid: { ...rule.grid, width: Number(event.target.value) } })} /></label>
              <label>Height<input type="number" min={80} value={rule.grid.height} onChange={(event) => updateRule({ ...rule, grid: { ...rule.grid, height: Number(event.target.value) } })} /></label>
            </div>
            <div className="inventory-mode-panel">
              <header><span><PackageOpen size={16} /></span><div><h3>All occupied slots</h3><p>Every occupied stash slot is eligible. Ctrl + click sends the item to the inventory.</p></div></header>
            </div>
          </div>
        </section>
      </section>

      <section className="workflow-section">
        <header><span><PackageOpen size={16} /></span><div><h3>Action timing</h3><p>One randomized range controls click waits, slot checks, and the pause after each moved slot.</p></div></header>
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

function overlayUrl(rule: StashInventoryRule) {
  const params = new URLSearchParams({
    view: 'stash-inventory-overlay',
    x: String(rule.grid.x),
    y: String(rule.grid.y),
    width: String(rule.grid.width),
    height: String(rule.grid.height),
    columns: String(rule.columns),
    rows: String(rule.rows),
  })
  return `/?${params.toString()}`
}

function physicalGridToLogicalWindow(grid: StashInventoryRule['grid']) {
  const scale = window.devicePixelRatio || 1
  return {
    x: Math.round(grid.x / scale),
    y: Math.round(grid.y / scale),
    width: Math.round(grid.width / scale),
    height: Math.round(grid.height / scale),
  }
}

function normalizeRule(rule?: StashInventoryRule): StashInventoryRule {
  const defaultRule: StashInventoryRule = {
    id: crypto.randomUUID(),
    name: 'Stash to inventory',
    enabled: false,
    triggerKey: 'F10',
    columns: 12,
    rows: 12,
    grid: { x: 18, y: 126, width: 632, height: 632 },
    emptyColor: '#17130f',
    tolerance: 18,
    humanization: { enabled: true, minMs: 120, maxMs: 240 },
  }
  return {
    ...defaultRule,
    ...rule,
    grid: { ...defaultRule.grid, ...rule?.grid },
    humanization: { ...defaultRule.humanization, ...rule?.humanization },
  }
}

function gridCount(value: string) {
  return Math.min(64, Math.max(1, Math.floor(Number(value) || 1)))
}

function testLabel(state: 'idle' | 'testing' | 'ready' | 'error', count?: number) {
  if (state === 'testing') return 'Checking...'
  if (state === 'ready') return `${count ?? 0} occupied`
  if (state === 'error') return 'Target unavailable'
  return 'Not tested'
}
