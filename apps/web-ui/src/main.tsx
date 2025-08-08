import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.tsx'
import './index.css'

const root = document.getElementById('root')

// biome-ignore lint/style/noNonNullAssertion: We know there is a root element.
createRoot(root!).render(
  <StrictMode>
    <App />
  </StrictMode>
)
