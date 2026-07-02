import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { InventoryGridOverlay } from './features/inventory-stash/InventoryGridOverlay.tsx'

const overlayView = new URLSearchParams(window.location.search).get('view')
const isInventoryOverlay = overlayView === 'inventory-overlay' || overlayView === 'tablet-scanner-overlay'
if (isInventoryOverlay) {
  document.documentElement.classList.add('inventory-overlay-root')
}

export const Root = isInventoryOverlay
  ? InventoryGridOverlay
  : App

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
)
