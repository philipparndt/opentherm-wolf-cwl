import { render } from 'preact'
import { App } from './App'
import './style.css'

// Light/dark follow the system live (index.html sets the initial state).
const mq = matchMedia('(prefers-color-scheme: dark)')
// In the Wall Display panel (html[data-embed=wall-display]) always dark.
mq.addEventListener('change', (e) => {
  const dark = e.matches || document.documentElement.dataset.embed === 'wall-display'
  document.documentElement.classList.toggle('dark', dark)
  document.documentElement.dataset.theme = dark ? 'dark' : 'light'
})

render(<App />, document.getElementById('app')!)
