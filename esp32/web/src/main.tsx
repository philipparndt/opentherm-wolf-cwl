import { render } from 'preact'
import { App } from './App'
import './style.css'

// Light/dark follow the system live (index.html sets the initial state).
const mq = matchMedia('(prefers-color-scheme: dark)')
mq.addEventListener('change', (e) => {
  document.documentElement.classList.toggle('dark', e.matches)
  document.documentElement.dataset.theme = e.matches ? 'dark' : 'light'
})

render(<App />, document.getElementById('app')!)
