import { createApp } from 'vue'
import App from './App.vue'
import router from './router'

import { createPinia } from 'pinia'

import 'ant-design-vue/dist/reset.css'
import * as Icons from '@ant-design/icons-vue'
import './style.css'


const app = createApp(App)
const pinia = createPinia()

for (const [key, component] of Object.entries(Icons)) {
    app.component(key, component)
}

app.use(pinia)
app.use(router)

app.mount('#app')