import React, { useEffect } from 'react'
import { ShowType } from '../lib/types'

export default function Menu({
  show,
  setShow,
  showMessage,
}: {
  show: ShowType | null
  setShow: Function
  showMessage: Function
}) {
  const [mp3sCount, setMp3sCount] = React.useState(0)
  const [playlistsCount, setPlaylistsCount] = React.useState(0)
  const [sourcesCount, setSourcesCount] = React.useState(0)

  function showMp3s(e: React.MouseEvent<HTMLLIElement, MouseEvent>) {
    e.preventDefault()
    showMessage('')
    setShow({ entity: 'mp3s', id: 0 })
  }

  function showPlaylists(e: React.MouseEvent<HTMLLIElement, MouseEvent>) {
    e.preventDefault()
    showMessage('')
    setShow({ entity: 'playlists', id: 0 })
  }

  function showSources(e: React.MouseEvent<HTMLLIElement, MouseEvent>) {
    e.preventDefault()
    showMessage('')
    setShow({ entity: 'sources', id: 0 })
  }

  function klasses(re: RegExp) {
    let klass = 'list-group-item list-group-item-action'
    klass += show && show['entity'].match(re) ? ' active' : ''
    return klass
  }

  function icon_klasses(icon: string, re: RegExp) {
    let klass = `fs-5 bi-${icon}`
    klass += show && show['entity'].match(re) ? ' text-white' : ''
    return klass
  }

  const getCounts = async () => {
    const response = await fetch('/api/counts')
    const data = await response.json()
    setMp3sCount(data.mp3s_count)
    setPlaylistsCount(data.playlists_count)
    setSourcesCount(data.sources_count)
  }

  useEffect(() => {
    getCounts()
  }, [])

  return (
    <ul className='list-group rounded-0'>
      <li className={`${klasses(/mp3s/)}`} onClick={showMp3s}>
        <i className={`${icon_klasses('music-note-beamed', /mp3s/)}`}></i>{' '}
        &nbsp;MP3s
        <span className='badge bg-dark text-light float-end mt-1'>
          {mp3sCount}
        </span>
      </li>
      <li className={`${klasses(/playlist/)}`} onClick={showPlaylists}>
        <i className={`${icon_klasses('card-list', /playlist/)}`}></i>{' '}
        &nbsp;Playlists
        <span className='badge bg-dark text-light float-end mt-1'>
          {playlistsCount}
        </span>
      </li>
      <li className={`${klasses(/source/)}`} onClick={showSources}>
        <i className={`${icon_klasses('folder', /source/)}`}></i> &nbsp;Sources
        <span className='badge bg-dark text-light float-end mt-1'>
          {sourcesCount}
        </span>
      </li>
    </ul>
  )
}
