import { invoke } from '@tauri-apps/api/core'

const hasTauriRuntime = () => '__TAURI_INTERNALS__' in window

export async function callBackend<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasTauriRuntime()) {
    return previewResponse<T>(command, args)
  }

  return invoke<T>(command, args)
}

function previewResponse<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (command === 'sample_pixel') {
    const request = args?.request as { x: number; y: number } | undefined
    return Promise.resolve({ color: '#34d399', x: request?.x ?? 0, y: request?.y ?? 0 } as T)
  }

  if (command === 'pick_pixel') {
    return Promise.resolve({ color: '#34d399', x: 640, y: 360 } as T)
  }

  if (command === 'test_pixel_rule') {
    return Promise.resolve(true as T)
  }

  if (command === 'test_inventory_stash_rule') {
    return Promise.resolve(18 as T)
  }

  if (command === 'capture_inventory_stash_snapshot') {
    const rule = args?.rule as { columns?: number; rows?: number } | undefined
    const columns = rule?.columns ?? 12
    const rows = rule?.rows ?? 5
    const snapshots = Array.from({ length: columns * rows }, (_, index) => ({
      slot: `${index % columns}:${Math.floor(index / columns)}`,
      color: index % 2 === 0 ? '#0f1110' : '#151923',
    }))
    return Promise.resolve(snapshots as T)
  }

  if (command === 'scan_tablet_stash') {
    return Promise.resolve({
      scannedSlots: 144,
      skippedSlots: ['0:1', '1:1'],
      tablets: [
        {
          slot: '2:3',
          column: 2,
          row: 3,
          tabletType: 'Abyss Precursor Tablet',
          rarity: 'Magic',
          usesRemaining: 10,
          valueTier: 'S',
          valueScore: 144,
          prefixes: [{ text: 'Map has 35% increased number of Rare Monsters', affixType: 'prefix', tier: 'A', score: 50 }],
          suffixes: [{ text: '2 additional Rare Monsters are spawned from Abysses in Map', affixType: 'suffix', tier: 'S', score: 76 }],
          unknownMods: [],
          reasons: ['S-tier 2 additional Rare Monsters are spawned from Abysses in Map', 'Prefix and suffix both have value', '10 uses remaining'],
          rawText: 'Abyss Precursor Tablet',
        },
        {
          slot: '5:4',
          column: 5,
          row: 4,
          tabletType: 'Ritual Precursor Tablet',
          rarity: 'Magic',
          usesRemaining: 10,
          valueTier: 'A',
          valueScore: 92,
          prefixes: [],
          suffixes: [{ text: 'Ritual Favours in Map have 65% increased chance to be Omens', affixType: 'suffix', tier: 'S', score: 74 }],
          unknownMods: [],
          reasons: ['S-tier Ritual Favours in Map have 65% increased chance to be Omens', 'Mechanic-specific suffix matches tablet type', '10 uses remaining'],
          rawText: 'Ritual Precursor Tablet',
        },
      ],
    } as T)
  }

  if (command === 'highlight_tablet_slot' || command === 'move_tablet_to_inventory') {
    return Promise.resolve(undefined as T)
  }

  if (command === 'validate_key_sequence') {
    return Promise.resolve({ valid: true, errors: [] } as T)
  }

  if (command === 'is_runtime_running') {
    return Promise.resolve(false as T)
  }

  if (command === 'get_foreground_app') {
    return Promise.resolve({ executable: 'Game.exe', path: 'C:\\Games\\Game.exe' } as T)
  }

  if (command === 'export_profile') {
    return Promise.resolve('{}' as T)
  }

  if (command === 'stop_macro_recording') {
    return Promise.resolve([] as T)
  }

  if (command === 'start_runtime' || command === 'stop_runtime' || command === 'save_profile' || command === 'delete_profile' || command === 'set_active_profile' || command === 'import_profile' || command === 'start_macro_recording' || command === 'test_pixel_actions') {
    return Promise.resolve(undefined as T)
  }

  return Promise.reject(new Error(`Preview mode does not implement ${command}`))
}
