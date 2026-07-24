import DefaultTheme from 'vitepress/theme'
import './custom.css'
import RullstSandbox from './components/RullstSandbox.vue'

export default {
  ...DefaultTheme,
  enhanceApp({ app }) {
    app.component('RullstSandbox', RullstSandbox)
  }
}
