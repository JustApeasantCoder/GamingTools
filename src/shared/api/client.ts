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
