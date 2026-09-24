import { mount } from 'svelte'
import '@fontsource/barlow/400.css'
import '@fontsource/barlow/500.css'
import '@fontsource/barlow/600.css'
import '@fontsource/barlow-condensed/500.css'
import '@fontsource/barlow-condensed/600.css'
import '@fontsource/barlow-condensed/700.css'
import './app.css'
import App from './App.svelte'
import { connect } from './lib/api/session'
import { applyUrlParams } from './lib/urlparams'

const session = await connect()
applyUrlParams()
const app = mount(App, { target: document.getElementById('app')!, props: { session } })

export default app
