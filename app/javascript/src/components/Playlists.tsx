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
    const req = await fetch('/api/playlists')
    const data = await req.json()
    setPlaylists(data.playlists)
    showWait(false)
  }

  useEffect(() => {
    getPlaylists()
  }, [])

  function editPlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    const id = e.currentTarget.id
    setShow({ entity: 'playlist', id: id })
  }

  function enqueuePlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    const id = e.currentTarget.id
    createQueuedMp3sFromPlaylist(id)
  }

  const doDeletePlaylist = async (id: string) => {
    showWait(true)
    const req = await fetch(`/api/playlists/${id}`, {
      method: 'DELETE',
      mode: 'cors',
      body: JSON.stringify({}),
      headers: {
        'Content-Type': 'application/json',
      },
    }).finally(() => {
      showWait(false)
    })

    const data = await req.json()

    if (data.message) {
      showMessage(data.message)
      getPlaylists()
    }
  }

  function deletePlaylist(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()

    if (window.confirm('Are you sure?')) {
      const id = e.currentTarget.id
      doDeletePlaylist(id)
    }
  }

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
                    <button
                      className='btn btn-sm btn-primary'
                      onClick={enqueuePlaylist}
                      id={playlist['id'].toString()}
                    >
                      Enqueue
                    </button>
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
