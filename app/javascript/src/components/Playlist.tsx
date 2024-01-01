import React, { useEffect, useState } from 'react'
import PlaylistMp3s from './PlaylistMp3s'
import { PlaylistType, NewPlaylistErrorType } from '../lib/types'

export default function Playlist({
  id,
  setShow,
  showMessage,
  showWait,
  createQueuedMp3,
  createQueuedMp3sFromPlaylist,
  setQ,
}: {
  id: number
  setShow: Function
  showMessage: Function
  showWait: Function
  createQueuedMp3: Function
  createQueuedMp3sFromPlaylist: Function
  setQ: Function
}) {
  const [playlist, setPlaylist] = useState<PlaylistType | null>(null)
  const [errors, setErrors] = useState<NewPlaylistErrorType>({ name: null })

  const getPlaylist = async () => {
    showWait(true)
    const req = await fetch(`/api/playlists/${id}`)
    const data = await req.json()
    setPlaylist(data.playlist)
    showWait(false)
  }

  useEffect(() => {
    getPlaylist()
  }, [id])

  const save = async () => {
    if (!playlist) return

    showWait(true)
    const req = await fetch(`/api/playlists/${id}`, {
      method: 'PUT',
      mode: 'cors',
      body: JSON.stringify({ playlist: { name: playlist.name } }),
      headers: {
        'Content-Type': 'application/json',
      },
    }).finally(() => {
      showWait(false)
    })

    const data = await req.json()

    if (data.errors) {
      setErrors(data.errors)
    } else {
      setPlaylist(data.playlist)
      setErrors({ name: null })
    }

    if (data.message) {
      showMessage(data.message)
    }
  }

  function savePlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    save()
  }

  function showPlaylists(
    e: React.MouseEvent<HTMLAnchorElement> | React.MouseEvent<HTMLButtonElement>
  ) {
    e.preventDefault()
    showMessage('')
    setShow({ entity: 'playlists', id: 0 })
  }

  function enqueuePlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    createQueuedMp3sFromPlaylist(id)
  }

  if (!playlist) return <></>

  return (
    <div className='container-fluid'>
      <nav aria-label='breadcrumb'>
        <ol className='breadcrumb'>
          <li className='breadcrumb-item'>
            <a
              href='#'
              onClick={showPlaylists}
              className='text-decoration-none'
            >
              <b>
                <i className='bi-card-list'></i> Playlists
              </b>
            </a>
          </li>
          <li className='breadcrumb-item' aria-current='page'>
            <b>{playlist['name']}</b>
          </li>
        </ol>
      </nav>

      <form className='form'>
        <div className='mb-3'>
          <label htmlFor='name' className='form-label'>
            <b>Name</b>
          </label>
          <input
            id='name'
            type='text'
            className='form-control'
            name='playlist[name]'
            defaultValue={playlist['name']}
            onChange={(e) => {
              setPlaylist({ ...playlist, name: e.target.value })
            }}
            onBlur={(e) => {
              setPlaylist({ ...playlist, name: e.target.value })
            }}
          />
          {errors.name && <div className='error'>{errors['name']}</div>}
        </div>
        <button
          type='submit'
          className='btn btn-primary'
          onClick={savePlaylist}
        >
          Save
        </button>{' '}
        <button
          type='submit'
          className='btn btn-primary'
          onClick={enqueuePlaylist}
        >
          Enqueue
        </button>
      </form>
      <PlaylistMp3s
        id={id}
        showWait={showWait}
        createQueuedMp3={createQueuedMp3}
        setQ={setQ}
        setShow={setShow}
      ></PlaylistMp3s>
    </div>
  )
}
