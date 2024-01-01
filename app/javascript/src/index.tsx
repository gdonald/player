import React from 'react'
import ReactDOM from 'react-dom/client'
import 'bootstrap/dist/js/bootstrap.bundle.min.js'
import 'bootstrap/dist/css/bootstrap.min.css'
import 'bootstrap-icons/font/bootstrap-icons.min.css'
import './css/index.css'
import App from './components/App'

const elem = document.getElementById('root')

if (elem === null || elem === undefined || !(elem instanceof HTMLElement)) {
  alert('Could not find root element')
} else {
  const root = ReactDOM.createRoot(elem)
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  )
}
