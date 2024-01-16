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
  const [mp3sCount, setMp3sCount] = useState<number>(0)

  const getPlaylist = async () => {
    showWait(true)
    await fetch(`/api/playlists/${id}`)
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        setPlaylist(data.playlist)
      })
      .finally(() => {
        showWait(false)
      })
  }

  const save = async () => {
    if (!playlist) return

    showWait(true)
    await fetch(`/api/playlists/${id}`, {
      method: 'PUT',
      mode: 'cors',
      body: JSON.stringify({ playlist: { name: playlist.name } }),
      headers: {
        'Content-Type': 'application/json',
      },
    })
      .then((req) => {
        if (!req.ok) {
          window.location.href = '/'
        }

        return req.json()
      })
      .then((data) => {
        if (data.errors) {
          setErrors(data.errors)
        } else {
          setPlaylist(data.playlist)
          setErrors({ name: null })
        }

        if (data.message) {
          showMessage(data.message)
        }
      })
      .catch((err) => {
        console.log(err)
      })
      .finally(() => {
        showWait(false)
      })
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

  useEffect(() => {
    getPlaylist()
  }, [id])

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
        {mp3sCount > 0 && (
          <button
            type='submit'
            className='btn btn-primary'
            onClick={enqueuePlaylist}
          >
            Enqueue
          </button>
        )}
      </form>
      <PlaylistMp3s
        id={id}
        showWait={showWait}
        createQueuedMp3={createQueuedMp3}
        setQ={setQ}
        setShow={setShow}
        setMp3sCount={setMp3sCount}
      ></PlaylistMp3s>
    </div>
  )
}
