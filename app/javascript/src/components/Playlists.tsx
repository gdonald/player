import React, { useState, useEffect } from 'react'
import { PlaylistType } from '../lib/types'

export default function Playlists({
  showWait,
  setShow,
  showMessage,
  createQueuedMp3sFromPlaylist,
}: {
  showWait: Function
  setShow: Function
  showMessage: Function
  createQueuedMp3sFromPlaylist: Function
}) {
  const [playlists, setPlaylists] = useState<PlaylistType[]>([])

  const getPlaylists = async () => {
    showWait(true)
    await fetch('/api/playlists')
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        setPlaylists(data.playlists)
      })
      .finally(() => {
        showWait(false)
      })
  }

  const doDeletePlaylist = async (id: string) => {
    showWait(true)
    await fetch(`/api/playlists/${id}`, {
      method: 'DELETE',
      mode: 'cors',
      body: JSON.stringify({}),
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
        if (data.message) {
          showMessage(data.message)
          getPlaylists()
        }
      })
      .finally(() => {
        showWait(false)
      })
  }

  function editPlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    setShow({ entity: 'playlist', id: e.currentTarget.id })
  }

  function enqueuePlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    createQueuedMp3sFromPlaylist(e.currentTarget.id)
  }

  function deletePlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()

    if (window.confirm('Are you sure?')) {
      doDeletePlaylist(e.currentTarget.id)
    }
  }

  useEffect(() => {
    getPlaylists()
  }, [])

  return (
    <div className='container-fluid'>
      <nav aria-label='breadcrumb'>
        <ol className='breadcrumb'>
          <li className='breadcrumb-item'>
            <b>
              <i className='bi-card-list'></i> Playlists
            </b>
          </li>
        </ol>
      </nav>

      {playlists.length === 0 && (
        <p className='text-center pt-5'>No playlists found.</p>
      )}

      {playlists.length > 0 && (
        <table className='table table-striped table-hover'>
          <thead>
            <tr>
              <th>Name</th>
              <th className='text-center'>MP3s</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {playlists.map((playlist) => (
              <tr key={`playlist-${playlist['id']}`} className='align-middle'>
                <td>{playlist['name']}</td>
                <td className='text-center'>{playlist['mp3s_count']}</td>
                <td className='tight'>
                  <div
                    className='btn-group'
                    role='group'
                    aria-label='Source actions'
                  >
                    {playlist['mp3s_count'] > 0 && (
                      <button
                        className='btn btn-sm btn-primary'
                        onClick={enqueuePlaylist}
                        id={playlist['id'].toString()}
                      >
                        Enqueue
                      </button>
                    )}
                    <button
                      className='btn btn-sm btn-primary'
                      onClick={editPlaylist}
                      id={playlist['id'].toString()}
                    >
                      Edit
                    </button>
                    <button
                      className='btn btn-sm btn-primary'
                      onClick={deletePlaylist}
                      id={playlist['id'].toString()}
                    >
                      Delete
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  )
}
