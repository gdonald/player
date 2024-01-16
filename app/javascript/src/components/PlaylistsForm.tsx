import React, { useEffect, useState } from 'react'
import {
  NewPlaylistType,
  PlaylistType,
  NewPlaylistErrorType,
} from '../lib/types'

export default function PlaylistsForm({
  showWait,
  showMessage,
  setShow,
  mp3Ids,
  playlists,
  q,
}: {
  showWait: Function
  showMessage: Function
  setShow: Function
  mp3Ids: string[]
  playlists: PlaylistType[]
  q: string | null
}) {
  const [showNewPlaylist, setShowNewPlaylist] = useState<boolean>(false)
  const [newPlaylist, setNewPlaylist] = useState<NewPlaylistType | null>(null)
  const [errors, setErrors] = useState<NewPlaylistErrorType | null>(null)

  const doUpdatePlaylist = async (id: string) => {
    showWait(true)
    await fetch(`/api/playlists/${id}`, {
      method: 'PUT',
      mode: 'cors',
      body: JSON.stringify({
        playlist: {
          playlist_mp3s_attributes: mp3Ids.map((mp3Id) => ({ mp3_id: mp3Id })),
        },
      }),
      headers: {
        'Content-Type': 'application/json',
      },
    })
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        if (data.errors) {
          setErrors(data.errors)
        } else {
          setErrors(null)
          setShow({ entity: 'playlist', id: data.playlist.id })
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

  function updatePlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    const id = document.querySelector('select')?.value
    if (!id) return

    doUpdatePlaylist(id)
  }

  function selectPlaylist(e: React.ChangeEvent<HTMLSelectElement>) {
    const playlistId = e.currentTarget.value
    if (playlistId === 'new') {
      let name = ''
      if (q) {
        const parts = q.split(':')
        if (parts.length == 2) {
          name = parts[1].replace(/"/g, '')
        }
      }

      setNewPlaylist({ ...newPlaylist, name: name })
      setShowNewPlaylist(true)
    }
  }

  const saveNewPlaylist = async () => {
    if (!newPlaylist) return

    showWait(true)
    await fetch(`/api/playlists`, {
      method: 'POST',
      mode: 'cors',
      body: JSON.stringify({
        playlist: {
          name: newPlaylist.name,
          playlist_mp3s_attributes: mp3Ids.map((mp3Id) => ({ mp3_id: mp3Id })),
        },
      }),
      headers: {
        'Content-Type': 'application/json',
      },
    })
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        if (data.errors) {
          setErrors(data.errors)
        } else {
          setErrors(null)
          setShow({ entity: 'playlist', id: data.playlist.id })
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

  function createNewPlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    saveNewPlaylist()
  }

  function cancelCreateNewPlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    setShowNewPlaylist(false)
  }

  return (
    <div className='playlists pt-1'>
      <form>
        {!showNewPlaylist && (
          <div className='input-group mb-0'>
            <select className='form-select' onChange={selectPlaylist}>
              <option key={`playlist-0}`} value={0}>
                Select Playlist
              </option>
              <option key={`playlist-new}`} value={'new'}>
                ** Create New Playlist **
              </option>
              {playlists.map((playlist) => (
                <option
                  key={`playlist-${playlist['id']}`}
                  value={playlist['id']}
                >
                  {playlist['name']}
                </option>
              ))}
            </select>
            <button
              type='submit'
              className='btn btn-primary'
              onClick={updatePlaylist}
            >
              Add to Playlist
            </button>
          </div>
        )}
        {showNewPlaylist && (
          <div className='input-group mb-0'>
            <input
              type='text'
              className='form-control'
              placeholder='Playlist Name'
              value={newPlaylist ? newPlaylist.name : ''}
              onChange={(e) => {
                setNewPlaylist({ ...newPlaylist, name: e.target.value })
              }}
              onBlur={(e) => {
                setNewPlaylist({ ...newPlaylist, name: e.target.value })
              }}
            />
            <button
              type='submit'
              className='btn btn-primary'
              onClick={cancelCreateNewPlaylist}
            >
              Cancel
            </button>
            <button
              type='submit'
              className='btn btn-primary'
              onClick={createNewPlaylist}
            >
              Create New Playlist
            </button>
            {errors && <div className='error'>{errors['name']}</div>}
          </div>
        )}
      </form>
    </div>
  )
}
