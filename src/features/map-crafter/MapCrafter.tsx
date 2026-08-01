import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { emitTo, listen } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Crosshair, ExternalLink, Grid3X3, Hammer, Search, ShieldCheck, X } from 'lucide-react'
import type {
  AppProfile,
  MapCraftReport,
  MapCrafterRule,
  MapScanReport,
  ScreenPoint,
} from '../../shared/types/profile'
import { callBackend } from '../../shared/api/client'
import { Button } from '../../shared/ui/Button'

interface MapCrafterProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
  onScan: (rule: MapCrafterRule) => Promise<MapScanReport>
  onCraft: (rule: MapCrafterRule) => Promise<MapCraftReport>
  onGetForegroundApp: () => Promise<{ executable: string; path: string }>
}

type CurrencyLocation = 'alchemy' | 'exalted' | 'scouring'

const currencyLabels: Record<CurrencyLocation, string> = {
  alchemy: 'Alchemy',
  exalted: 'Exalted',
  scouring: 'Scouring',
}

export function MapCrafter({ profile, onProfileChange, onScan, onCraft, onGetForegroundApp }: MapCrafterProps) {
  const rule = normalizeRule(profile.mapCrafterRules?.[0])
  const [scanState, setScanState] = useState<'idle' | 'scanning' | 'ready' | 'error'>('idle')
  const [craftState, setCraftState] = useState<'idle' | 'crafting' | 'ready' | 'error'>('idle')
  const [report, setReport] = useState<MapScanReport>()
  const [craftReport, setCraftReport] = useState<MapCraftReport>()
  const [error, setError] = useState<string>()
  const [captureState, setCaptureState] = useState<'idle' | 'waiting' | 'error'>('idle')
  const [locationCapture, setLocationCapture] = useState<CurrencyLocation>()
  const [overlayOpen, setOverlayOpen] = useState(false)
  const [overlayError, setOverlayError] = useState<string>()
  const ruleRef = useRef(rule)
  const targetRule = useMemo(() => ({
    ...rule,
    targetExecutable: rule.targetExecutable || profile.runtimeSettings.foregroundGuard.executable,
  }), [profile.runtimeSettings.foregroundGuard.executable, rule])

  const updateRule = useCallback((nextRule: MapCrafterRule) => {
    onProfileChange({ ...profile, mapCrafterRules: [nextRule] })
  }, [onProfileChange, profile])
  const updateRuleRef = useRef(updateRule)

  useEffect(() => {
    ruleRef.current = rule
    updateRuleRef.current = updateRule
  }, [rule, updateRule])

  useEffect(() => {
    if (!hasTauriRuntime()) return
    let cancelled = false
    let unlistenGrid: (() => void) | undefined
    let unlistenReady: (() => void) | undefined
    let unlistenClosed: (() => void) | undefined

    void listen<MapCrafterRule['grid']>('map-crafter-overlay-grid-change', (event) => {
      updateRuleRef.current({ ...ruleRef.current, grid: event.payload })
    }).then((dispose) => {
      if (cancelled) dispose()
      else unlistenGrid = dispose
    })
    void listen('map-crafter-overlay-ready', () => {
      setOverlayOpen(true)
      void emitTo('map-crafter-overlay', 'map-crafter-overlay-config', ruleRef.current)
    }).then((dispose) => {
      if (cancelled) dispose()
      else unlistenReady = dispose
    })
    void listen('map-crafter-overlay-closed', () => {
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

  const openOverlay = async () => {
    setOverlayError(undefined)
    if (!hasTauriRuntime()) {
      setOverlayError('Screen overlay is available in the desktop app.')
      return
    }

    const existing = await WebviewWindow.getByLabel('map-crafter-overlay')
    await existing?.close().catch(() => undefined)
    const overlay = new WebviewWindow('map-crafter-overlay', {
      url: overlayUrl(rule),
      title: 'Map Crafter Grid Overlay',
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
      void emitTo('map-crafter-overlay', 'map-crafter-overlay-config', ruleRef.current)
    })
    overlay.once('tauri://error', (event) => {
      setOverlayError(String(event.payload))
      setOverlayOpen(false)
    })
  }

  const closeOverlay = async () => {
    const existing = await WebviewWindow.getByLabel('map-crafter-overlay')
    await existing?.close()
    setOverlayOpen(false)
  }

  const scan = async () => {
    setScanState('scanning')
    setError(undefined)
    try {
      const nextReport = await onScan(targetRule)
      setReport(nextReport)
      setScanState('ready')
    } catch (nextError) {
      setError(errorMessage(nextError))
      setScanState('error')
    }
  }

  const craft = async () => {
    setCraftState('crafting')
    setError(undefined)
    try {
      const nextReport = await onCraft(targetRule)
      setCraftReport(nextReport)
      setReport(nextReport.initialScan)
      setCraftState('ready')
      setScanState('ready')
    } catch (nextError) {
      setError(errorMessage(nextError))
      setCraftState('error')
    }
  }

  const captureTarget = () => {
    setCaptureState('waiting')
    setTimeout(() => {
      onGetForegroundApp()
        .then((app) => {
          updateRule({ ...ruleRef.current, targetExecutable: app.executable })
          setCaptureState('idle')
        })
        .catch(() => setCaptureState('error'))
    }, 2500)
  }

  const captureCurrency = async (currency: CurrencyLocation) => {
    setLocationCapture(currency)
    try {
      const point = await callBackend<ScreenPoint>('capture_map_currency_location', { waitMs: 2500 })
      updateRule({ ...ruleRef.current, craft: { ...ruleRef.current.craft, [currency]: point } })
    } catch (nextError) {
      setError(errorMessage(nextError))
    } finally {
      setLocationCapture(undefined)
    }
  }

  return (
    <div className="feature-surface tablet-scanner map-crafter">
      <section className="macro-summary">
        <div>
          <h2>{rule.name}</h2>
          <p>
            <span>{rule.columns} x {rule.rows}</span>
            <span>{targetRule.targetExecutable || 'No target app'}</span>
            <span>{report ? `${report.maps.length} maps found` : `${rule.columns * rule.rows} slots`}</span>
            <span>{rule.scanDelayMs} ms copy wait</span>
            <span>Empty tolerance {rule.emptyTolerance}</span>
          </p>
        </div>
        <div className="toolbar-group">
          <span className={`test-status ${scanState === 'error' ? 'error' : scanState === 'ready' ? 'matching' : ''}`}>
            {scanLabel(scanState, report)}
          </span>
          <Button icon={Search} onClick={scan} disabled={scanState === 'scanning' || craftState === 'crafting'}>
            {scanState === 'scanning' ? 'Scanning...' : 'Scan grid'}
          </Button>
          <Button icon={Hammer} variant="primary" onClick={craft} disabled={craftState === 'crafting' || scanState === 'scanning'}>
            {craftState === 'crafting' ? 'Crafting...' : 'Craft maps'}
          </Button>
        </div>
      </section>

      <section className="workflow-section">
        <header><span>1</span><div><h3>Map grid</h3><p>Match the overlay to the visible map slots. The cursor parks below the grid while all empty slots are detected, before any map tooltip opens.</p></div></header>
        <div className="inventory-overlay-row tablet-overlay-row">
          <div className="inventory-overlay-launcher">
            <Grid3X3 size={32} />
            <strong>{overlayOpen ? 'Screen overlay is open' : 'Screen overlay is closed'}</strong>
            <span>Drag and resize the grid over the map slots that should be scanned and crafted.</span>
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
          <div className="inventory-control-grid compact tablet-grid-settings">
            <label>Columns<input type="number" min={1} max={24} value={rule.columns} onChange={(event) => updateRule({ ...rule, columns: clamp(Number(event.target.value), 1, 24) })} /></label>
            <label>Rows<input type="number" min={1} max={24} value={rule.rows} onChange={(event) => updateRule({ ...rule, rows: clamp(Number(event.target.value), 1, 24) })} /></label>
            <label>Copy wait ms<input type="number" min={20} max={1000} value={rule.scanDelayMs} onChange={(event) => updateRule({ ...rule, scanDelayMs: clamp(Number(event.target.value), 20, 1000) })} /></label>
            <label>Empty tolerance<input type="number" min={0} max={64} value={rule.emptyTolerance} onChange={(event) => updateRule({ ...rule, emptyTolerance: clamp(Number(event.target.value), 0, 64) })} /></label>
            <label>Target app<input value={rule.targetExecutable} onChange={(event) => updateRule({ ...rule, targetExecutable: event.target.value })} placeholder="PathOfExileSteam.exe" /></label>
            <div className="inventory-button-stack">
              <Button icon={Crosshair} onClick={captureTarget} disabled={captureState === 'waiting'}>
                {captureState === 'waiting' ? 'Switch to game...' : 'Capture target'}
              </Button>
            </div>
          </div>
        </div>
        {captureState === 'error' ? <div className="notice notice-error">Could not capture the foreground app.</div> : null}
      </section>

      <section className="workflow-section">
        <header><span>2</span><div><h3>Currency picker</h3><p>Press Pick, then place the cursor over that currency in the player inventory. The location is captured after 2.5 seconds.</p></div></header>
        <div className="tablet-craft-grid">
          {(Object.keys(currencyLabels) as CurrencyLocation[]).map((currency) => (
            <label key={currency}>
              {currencyLabels[currency]}
              <div className="tablet-location-row">
                <code>{pointLabel(rule.craft[currency])}</code>
                <Button icon={Crosshair} onClick={() => captureCurrency(currency)} disabled={locationCapture !== undefined}>
                  {locationCapture === currency ? 'Place cursor...' : 'Pick'}
                </Button>
              </div>
            </label>
          ))}
          <label>Craft wait ms<input type="number" min={20} max={2000} value={rule.craft.craftDelayMs} onChange={(event) => updateRule({ ...rule, craft: { ...rule.craft, craftDelayMs: clamp(Number(event.target.value), 20, 2000) } })} /></label>
        </div>
      </section>

      <section className="workflow-section">
        <header><span><ShieldCheck size={16} /></span><div><h3>Craft sequence</h3><p>Normal and Magic maps receive Alchemy first. Rare maps go directly to two Exalted applications; Unique maps are skipped.</p></div></header>
        <div className="map-craft-sequence">
          <span>Magic only: Scouring</span>
          <strong>→</strong>
          <span>Alchemy</span>
          <strong>→</strong>
          <span>Exalted</span>
          <strong>→</strong>
          <span>Exalted</span>
        </div>
        {error ? <div className="notice notice-error">{error}</div> : null}
        {craftReport ? (
          <div className="notice">
            Crafted {craftReport.craftedSlots.length} map{craftReport.craftedSlots.length === 1 ? '' : 's'} with {craftReport.actions.length} currency action{craftReport.actions.length === 1 ? '' : 's'}; skipped {craftReport.skippedSlots.length} slot{craftReport.skippedSlots.length === 1 ? '' : 's'}.
          </div>
        ) : null}
        {report ? <MapResults report={report} columns={rule.columns} rows={rule.rows} /> : <div className="empty-panel">Scan results will appear here before or after crafting.</div>}
      </section>
    </div>
  )
}

function MapResults({ report, columns, rows }: { report: MapScanReport; columns: number; rows: number }) {
  const bySlot = new Map(report.maps.map((map) => [map.slot, map]))
  const counts = report.maps.reduce<Record<string, number>>((result, map) => {
    result[map.rarity] = (result[map.rarity] ?? 0) + 1
    return result
  }, {})

  return (
    <div className="map-results">
      <div className="map-result-summary">
        {Object.entries(counts).map(([rarity, count]) => <span key={rarity}>{rarity}: {count}</span>)}
        <span>Empty / non-map: {report.skippedSlots.length}</span>
      </div>
      <div className="map-result-grid" style={{ gridTemplateColumns: `repeat(${columns}, minmax(24px, 1fr))` }}>
        {Array.from({ length: columns * rows }, (_, index) => {
          const column = index % columns
          const row = Math.floor(index / columns)
          const slot = `${column}:${row}`
          const map = bySlot.get(slot)
          return (
            <span
              key={slot}
              className={`map-result-slot ${map ? map.rarity.toLowerCase() : 'empty'}`}
              title={map ? `${map.rarity} · ${map.name ?? map.itemType}` : `Empty / non-map · ${column + 1},${row + 1}`}
            >
              {column + 1},{row + 1}
            </span>
          )
        })}
      </div>
    </div>
  )
}

function normalizeRule(rule?: MapCrafterRule): MapCrafterRule {
  const defaultRule: MapCrafterRule = {
    id: crypto.randomUUID(),
    name: 'Map crafter',
    targetExecutable: '',
    columns: 12,
    rows: 6,
    grid: { x: 18, y: 126, width: 632, height: 316 },
    scanDelayMs: 90,
    emptyTolerance: 8,
    craft: {
      alchemy: { x: 0, y: 0 },
      exalted: { x: 0, y: 0 },
      scouring: { x: 0, y: 0 },
      craftDelayMs: 90,
    },
  }
  return {
    ...defaultRule,
    ...rule,
    grid: { ...defaultRule.grid, ...rule?.grid },
    craft: { ...defaultRule.craft, ...rule?.craft },
  }
}

function overlayUrl(rule: MapCrafterRule) {
  const params = new URLSearchParams({
    view: 'map-crafter-overlay',
    x: String(rule.grid.x),
    y: String(rule.grid.y),
    width: String(rule.grid.width),
    height: String(rule.grid.height),
    columns: String(rule.columns),
    rows: String(rule.rows),
  })
  return `/?${params.toString()}`
}

function physicalGridToLogicalWindow(grid: MapCrafterRule['grid']) {
  const scale = window.devicePixelRatio || 1
  return {
    x: Math.round(grid.x / scale),
    y: Math.round(grid.y / scale),
    width: Math.round(grid.width / scale),
    height: Math.round(grid.height / scale),
  }
}

function scanLabel(state: 'idle' | 'scanning' | 'ready' | 'error', report?: MapScanReport) {
  if (state === 'scanning') return 'Scanning...'
  if (state === 'ready') return `${report?.maps.length ?? 0} found`
  if (state === 'error') return 'Scan failed'
  return 'Not scanned'
}

function pointLabel(point: ScreenPoint) {
  return point.x === 0 && point.y === 0 ? 'Not picked' : `${point.x}, ${point.y}`
}

function hasTauriRuntime() {
  return '__TAURI_INTERNALS__' in window
}

function clamp(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) return min
  return Math.min(max, Math.max(min, value))
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}
