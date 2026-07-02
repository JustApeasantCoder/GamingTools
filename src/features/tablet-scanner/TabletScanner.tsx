import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { emitTo, listen } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Crosshair, ExternalLink, Grid3X3, Hammer, PackagePlus, Plus, Search, ShieldCheck, Trash2, X } from 'lucide-react'
import type { AppProfile, ScreenPoint, TabletCraftReport, TabletScanEvent, TabletScanReport, TabletScannerRule, TabletValueRuleConfig } from '../../shared/types/profile'
import { callBackend } from '../../shared/api/client'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'

interface TabletScannerProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
  onScan: (rule: TabletScannerRule) => Promise<TabletScanReport>
  onScanAndCraft: (rule: TabletScannerRule) => Promise<TabletCraftReport>
  onHighlightSlot: (rule: TabletScannerRule, slot: string) => Promise<void>
  onMoveToInventory: (rule: TabletScannerRule, slot: string) => Promise<void>
  onGetForegroundApp: () => Promise<{ executable: string; path: string }>
}

type CraftCurrency = keyof TabletScannerRule['craft']
type CraftLocationKey = Exclude<CraftCurrency, 'tabSwitchDelayMs' | 'craftDelayMs'>

const craftCurrencyLabels: Record<CraftLocationKey, string> = {
  transmutation: 'Transmutation',
  augmentation: 'Augmentation',
  regal: 'Regal',
  exalted: 'Exalted',
  alchemy: 'Alchemy',
}

export function TabletScanner({ profile, onProfileChange, onScan, onScanAndCraft, onHighlightSlot, onMoveToInventory, onGetForegroundApp }: TabletScannerProps) {
  const rule = normalizeRule(profile.tabletScannerRules?.[0])
  const [scanState, setScanState] = useState<'idle' | 'scanning' | 'ready' | 'error'>('idle')
  const [craftState, setCraftState] = useState<'idle' | 'crafting' | 'ready' | 'error'>('idle')
  const [report, setReport] = useState<TabletScanReport>()
  const [craftReport, setCraftReport] = useState<TabletCraftReport>()
  const [scanError, setScanError] = useState<string>()
  const [craftError, setCraftError] = useState<string>()
  const [selectedSlot, setSelectedSlot] = useState<string>()
  const [captureState, setCaptureState] = useState<'idle' | 'waiting' | 'error'>('idle')
  const [locationCapture, setLocationCapture] = useState<CraftLocationKey | undefined>()
  const [overlayOpen, setOverlayOpen] = useState(false)
  const [overlayError, setOverlayError] = useState<string>()
  const ruleRef = useRef(rule)
  const targetRule = useMemo(() => ({
    ...rule,
    targetExecutable: rule.targetExecutable || profile.runtimeSettings.foregroundGuard.executable,
  }), [profile.runtimeSettings.foregroundGuard.executable, rule])
  const slots = useMemo(() => createSlots(rule.columns, rule.rows), [rule.columns, rule.rows])
  const valuableCount = report?.tablets.filter((tablet) => tablet.valueTier !== 'Low').length ?? 0

  const updateRule = useCallback((nextRule: TabletScannerRule) => {
    onProfileChange({ ...profile, tabletScannerRules: [nextRule] })
  }, [onProfileChange, profile])

  useEffect(() => {
    ruleRef.current = rule
  }, [rule])

  useEffect(() => {
    if (!hasTauriRuntime()) return
    let unlistenGrid: (() => void) | undefined
    let unlistenReady: (() => void) | undefined
    let unlistenClosed: (() => void) | undefined

    void listen<TabletScannerRule['grid']>('tablet-scanner-overlay-grid-change', (event) => {
      updateRule({ ...ruleRef.current, grid: event.payload })
    }).then((dispose) => {
      unlistenGrid = dispose
    })

    void listen('tablet-scanner-overlay-ready', () => {
      setOverlayOpen(true)
      void emitTo('tablet-scanner-overlay', 'tablet-scanner-overlay-config', ruleRef.current)
    }).then((dispose) => {
      unlistenReady = dispose
    })

    void listen('tablet-scanner-overlay-closed', () => {
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

  useEffect(() => {
    if (!hasTauriRuntime()) return
    let unlistenScan: (() => void) | undefined
    void listen<TabletScanEvent>('tablet-scan-report', (event) => {
      if (event.payload.ruleId !== ruleRef.current.id) return
      setReport(event.payload.report)
      setScanState('ready')
      setScanError(undefined)
    }).then((dispose) => {
      unlistenScan = dispose
    })
    return () => {
      unlistenScan?.()
    }
  }, [])

  const openOverlay = async () => {
    setOverlayError(undefined)
    if (!hasTauriRuntime()) {
      setOverlayError('Screen overlay is available in the desktop app.')
      return
    }

    const existing = await WebviewWindow.getByLabel('tablet-scanner-overlay')
    await existing?.close().catch(() => undefined)

    const overlay = new WebviewWindow('tablet-scanner-overlay', {
      url: overlayUrl(rule),
      title: 'Tablet Scanner Grid Overlay',
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
      void emitTo('tablet-scanner-overlay', 'tablet-scanner-overlay-config', ruleRef.current)
    })
    overlay.once('tauri://error', (event) => {
      setOverlayError(String(event.payload))
      setOverlayOpen(false)
    })
  }

  const closeOverlay = async () => {
    const existing = await WebviewWindow.getByLabel('tablet-scanner-overlay')
    await existing?.close()
    setOverlayOpen(false)
  }

  const scan = async () => {
    setScanState('scanning')
    setScanError(undefined)
    try {
      const nextReport = await onScan(targetRule)
      setReport(nextReport)
      setScanState('ready')
    } catch (error) {
      setScanError(error instanceof Error ? error.message : String(error))
      setScanState('error')
    }
  }

  const scanAndCraft = async () => {
    setCraftState('crafting')
    setCraftError(undefined)
    try {
      const nextReport = await onScanAndCraft(targetRule)
      setCraftReport(nextReport)
      setReport(nextReport.finalScan)
      setScanState('ready')
      setCraftState('ready')
    } catch (error) {
      setCraftError(error instanceof Error ? error.message : String(error))
      setCraftState('error')
    }
  }

  const captureTarget = async () => {
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

  const highlightSlot = async (slot: string) => {
    setSelectedSlot(slot)
    await onHighlightSlot(targetRule, slot)
  }

  const moveToInventory = async (slot: string) => {
    setSelectedSlot(slot)
    await onMoveToInventory(targetRule, slot)
  }

  const captureCraftLocation = async (currency: CraftLocationKey) => {
    setLocationCapture(currency)
    try {
      const point = await callBackend<ScreenPoint>('capture_tablet_craft_location', { waitMs: 2500 })
      updateRule({ ...ruleRef.current, craft: { ...ruleRef.current.craft, [currency]: point } })
    } finally {
      setLocationCapture(undefined)
    }
  }

  const addValueRule = () => {
    updateRule({
      ...rule,
      valueRules: [
        ...rule.valueRules,
        {
          id: crypto.randomUUID(),
          label: 'Custom roll',
          tabletMatch: '',
          textMatch: '',
          affixType: 'suffix',
          tier: 'A',
          score: 40,
        },
      ],
    })
  }

  const updateValueRule = (id: string, patch: Partial<TabletValueRuleConfig>) => {
    updateRule({
      ...rule,
      valueRules: rule.valueRules.map((item) => item.id === id ? { ...item, ...patch } : item),
    })
  }

  const removeValueRule = (id: string) => {
    updateRule({ ...rule, valueRules: rule.valueRules.filter((item) => item.id !== id) })
  }

  return (
    <div className="feature-surface tablet-scanner">
      <section className="macro-summary">
        <div>
          <h2>{rule.name}</h2>
          <p>
            <span>{rule.columns} x {rule.rows}</span>
            <span>{rule.triggerKey}</span>
            <span>{targetRule.targetExecutable || 'No target app'}</span>
            <span>{report ? `${report.tablets.length} tablets found` : `${slots.length} slots`}</span>
            <span>{report ? `${valuableCount} worth checking` : `${rule.scanDelayMs} ms copy wait`}</span>
          </p>
        </div>
        <div className="toolbar-group">
          <span className={`test-status ${scanState === 'error' ? 'error' : scanState === 'ready' ? 'matching' : ''}`}>{scanLabel(scanState, report)}</span>
          <Button icon={Search} variant="primary" onClick={scan} disabled={scanState === 'scanning'}>
            {scanState === 'scanning' ? 'Scanning...' : 'Scan stash'}
          </Button>
          <Button icon={Hammer} variant="primary" onClick={scanAndCraft} disabled={craftState === 'crafting'}>
            {craftState === 'crafting' ? 'Crafting...' : 'Scan and craft'}
          </Button>
        </div>
      </section>

      <section className="workflow-section">
        <header><span>1</span><div><h3>Stash grid</h3><p>Match the overlay to the visible tablet stash tab before scanning.</p></div></header>
        <div className="inventory-overlay-row tablet-overlay-row">
          <div className="inventory-overlay-launcher">
            <Grid3X3 size={32} />
            <strong>{overlayOpen ? 'Screen overlay is open' : 'Screen overlay is closed'}</strong>
            <span>Drag and resize the grid over the stash slots that should be scanned.</span>
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
            <label>Scan hotkey<KeyCaptureButton value={rule.triggerKey} onChange={(triggerKey) => updateRule({ ...rule, triggerKey })} label="Change scan hotkey" /></label>
            <label>Columns<input type="number" min={1} max={24} value={rule.columns} onChange={(event) => updateRule({ ...rule, columns: clamp(Number(event.target.value), 1, 24) })} /></label>
            <label>Rows<input type="number" min={1} max={24} value={rule.rows} onChange={(event) => updateRule({ ...rule, rows: clamp(Number(event.target.value), 1, 24) })} /></label>
            <label>Copy wait ms<input type="number" min={20} max={1000} value={rule.scanDelayMs} onChange={(event) => updateRule({ ...rule, scanDelayMs: clamp(Number(event.target.value), 20, 1000) })} /></label>
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
        <header><span><ShieldCheck size={16} /></span><div><h3>Scan behavior</h3><p>Moves the cursor over each configured slot, copies the hovered item text, then ranks known valuable tablet rolls locally.</p></div></header>
        {scanError ? <div className="notice notice-error">{scanError}</div> : null}
        {craftError ? <div className="notice notice-error">{craftError}</div> : null}
        {craftReport ? <div className="notice">{craftReport.actions.length} craft action{craftReport.actions.length === 1 ? '' : 's'} completed. Final scan found {craftReport.finalScan.tablets.length} tablet{craftReport.finalScan.tablets.length === 1 ? '' : 's'}.</div> : null}
        {report ? <TabletResults report={report} selectedSlot={selectedSlot} onHighlightSlot={highlightSlot} onMoveToInventory={moveToInventory} /> : <div className="empty-panel">Scan results will appear here.</div>}
      </section>

      <section className="workflow-section">
        <header><span>2</span><div><h3>Craft setup</h3><p>Pick each currency location, then the scanner can switch tabs and apply currency in efficient passes.</p></div></header>
        <div className="tablet-craft-grid">
          {(Object.keys(craftCurrencyLabels) as CraftLocationKey[]).map((currency) => (
            <label key={currency}>
              {craftCurrencyLabels[currency]}
              <div className="tablet-location-row">
                <code>{pointLabel(rule.craft[currency])}</code>
                <Button icon={Crosshair} onClick={() => captureCraftLocation(currency)} disabled={locationCapture !== undefined}>
                  {locationCapture === currency ? 'Place cursor...' : 'Pick'}
                </Button>
              </div>
            </label>
          ))}
          <label>Tab wait ms<input type="number" min={20} max={1000} value={rule.craft.tabSwitchDelayMs} onChange={(event) => updateRule({ ...rule, craft: { ...rule.craft, tabSwitchDelayMs: clamp(Number(event.target.value), 20, 1000) } })} /></label>
          <label>Craft wait ms<input type="number" min={20} max={2000} value={rule.craft.craftDelayMs} onChange={(event) => updateRule({ ...rule, craft: { ...rule.craft, craftDelayMs: clamp(Number(event.target.value), 20, 2000) } })} /></label>
        </div>
      </section>

      <section className="workflow-section">
        <header><span>3</span><div><h3>Tier list</h3><p>Add custom roll matches for the scanner and craft decisions. Built-in tablet rolls still apply.</p></div></header>
        <div className="tablet-tier-list">
          <div className="tablet-tier-head">
            <span>Name</span>
            <span>Tablet contains</span>
            <span>Mod contains</span>
            <span>Type</span>
            <span>Tier</span>
            <span>Score</span>
            <span></span>
          </div>
          {rule.valueRules.map((valueRule) => (
            <div className="tablet-tier-row" key={valueRule.id}>
              <input value={valueRule.label} onChange={(event) => updateValueRule(valueRule.id, { label: event.target.value })} />
              <input value={valueRule.tabletMatch} onChange={(event) => updateValueRule(valueRule.id, { tabletMatch: event.target.value })} placeholder="optional" />
              <input value={valueRule.textMatch} onChange={(event) => updateValueRule(valueRule.id, { textMatch: event.target.value })} placeholder="required text" />
              <select value={valueRule.affixType} onChange={(event) => updateValueRule(valueRule.id, { affixType: event.target.value as TabletValueRuleConfig['affixType'] })}>
                <option value="prefix">Prefix</option>
                <option value="suffix">Suffix</option>
              </select>
              <select value={valueRule.tier} onChange={(event) => updateValueRule(valueRule.id, { tier: event.target.value as TabletValueRuleConfig['tier'] })}>
                <option value="S">S</option>
                <option value="A">A</option>
                <option value="B">B</option>
              </select>
              <input type="number" min={1} max={200} value={valueRule.score} onChange={(event) => updateValueRule(valueRule.id, { score: clamp(Number(event.target.value), 1, 200) })} />
              <Button icon={Trash2} onClick={() => removeValueRule(valueRule.id)} title="Remove custom roll">Remove</Button>
            </div>
          ))}
          {rule.valueRules.length === 0 ? <div className="empty-panel">No custom rolls yet.</div> : null}
          <Button icon={Plus} onClick={addValueRule}>Add custom roll</Button>
        </div>
      </section>
    </div>
  )
}

function TabletResults({ report, selectedSlot, onHighlightSlot, onMoveToInventory }: { report: TabletScanReport; selectedSlot?: string; onHighlightSlot: (slot: string) => void; onMoveToInventory: (slot: string) => void }) {
  return (
    <div className="tablet-results">
      <div className="tablet-results-head">
        <span>Slot</span>
        <span>Tablet</span>
        <span>Value</span>
        <span>Reasons</span>
        <span>Action</span>
      </div>
      {report.tablets.map((tablet) => (
        <article key={`${tablet.slot}-${tablet.rawText}`} className={`tablet-result-card tier-${tablet.valueTier.toLowerCase()} ${selectedSlot === tablet.slot ? 'selected' : ''}`} onClick={() => onHighlightSlot(tablet.slot)}>
          <div><kbd>{slotLabel(tablet.slot)}</kbd></div>
          <div className="tablet-result-main">
            <strong>{tablet.name ? `${tablet.name} ${tablet.tabletType}` : tablet.tabletType}</strong>
            <span>{tablet.rarity}{tablet.usesRemaining !== undefined ? ` / ${tablet.usesRemaining} uses` : ''}</span>
            <div className="tablet-mod-list">
              {[...tablet.prefixes, ...tablet.suffixes].map((modifier) => (
                <span key={`${tablet.slot}-${modifier.text}`} className={`tablet-mod tier-${modifier.tier.toLowerCase()}`}>
                  {modifier.tier} {modifier.affixType}: {modifier.text}
                </span>
              ))}
            </div>
          </div>
          <div><span className={`tablet-value-badge tier-${tablet.valueTier.toLowerCase()}`}>{tablet.valueTier} / {tablet.valueScore}</span></div>
          <ul>
            {tablet.reasons.slice(0, 4).map((reason) => <li key={reason}>{reason}</li>)}
          </ul>
          <div className="tablet-result-actions">
            <Button
              icon={PackagePlus}
              onClick={(event) => {
                event.stopPropagation()
                onMoveToInventory(tablet.slot)
              }}
            >
              Move to inventory
            </Button>
          </div>
        </article>
      ))}
      {report.tablets.length === 0 ? <div className="empty-panel">No tablets were detected in the scanned slots.</div> : null}
    </div>
  )
}

function hasTauriRuntime() {
  return '__TAURI_INTERNALS__' in window
}

function normalizeRule(rule?: TabletScannerRule): TabletScannerRule {
  const defaultRule: TabletScannerRule = {
    id: crypto.randomUUID(),
    name: 'Tablet stash scanner',
    triggerKey: 'F9',
    targetExecutable: '',
    columns: 12,
    rows: 12,
    grid: { x: 18, y: 126, width: 632, height: 632 },
    scanDelayMs: 90,
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
  return {
    ...defaultRule,
    ...rule,
    grid: { ...defaultRule.grid, ...rule?.grid },
    craft: { ...defaultRule.craft, ...rule?.craft },
    valueRules: rule?.valueRules ?? [],
  }
}

function overlayUrl(rule: TabletScannerRule) {
  const params = new URLSearchParams({
    view: 'tablet-scanner-overlay',
    x: String(rule.grid.x),
    y: String(rule.grid.y),
    width: String(rule.grid.width),
    height: String(rule.grid.height),
    columns: String(rule.columns),
    rows: String(rule.rows),
  })
  return `/?${params.toString()}`
}

function physicalGridToLogicalWindow(grid: TabletScannerRule['grid']) {
  const scale = window.devicePixelRatio || 1
  return {
    x: Math.round(grid.x / scale),
    y: Math.round(grid.y / scale),
    width: Math.round(grid.width / scale),
    height: Math.round(grid.height / scale),
  }
}

function createSlots(columns: number, rows: number) {
  return Array.from({ length: columns * rows })
}

function scanLabel(state: 'idle' | 'scanning' | 'ready' | 'error', report?: TabletScanReport) {
  if (state === 'scanning') return 'Scanning...'
  if (state === 'ready') return `${report?.tablets.length ?? 0} found`
  if (state === 'error') return 'Scan failed'
  return 'Not scanned'
}

function slotLabel(slot: string) {
  const [column, row] = slot.split(':').map(Number)
  if (!Number.isFinite(column) || !Number.isFinite(row)) return slot
  return `${column + 1},${row + 1}`
}

function clamp(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) return min
  return Math.min(max, Math.max(min, value))
}

function pointLabel(point: ScreenPoint) {
  if (!point.x && !point.y) return 'Not picked'
  return `${point.x}, ${point.y}`
}
