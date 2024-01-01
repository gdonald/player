import React, { useEffect, useState } from 'react'
import { PlaylistMp3Type } from '../lib/types'

export default function PlaylistMp3s({
  id,
  showWait,
  createQueuedMp3,
  setQ,
  setShow,
}: {
  id: number
  showWait: Function
  createQueuedMp3: Function
  setQ: Function
  setShow: Function
}) {
  const [playlistMp3s, setPlaylistMp3s] = useState<PlaylistMp3Type[]>([])

  const getMp3s = async () => {
    showWait(true)
    const req = await fetch(`/api/playlists/${id}/playlist_mp3s`)
    const data = await req.json()
    setPlaylistMp3s(data.playlist_mp3s)
    showWait(false)
  }

  useEffect(() => {
    getMp3s()
  }, [])

  function playMp3(e: React.MouseEvent<HTMLAnchorElement>) {
    e.preventDefault()
    const id = e.currentTarget.id
    createQueuedMp3(id)
  }

  const doMoveHigher = async (playlist_mp3_id: string) => {
    showWait(true)
    const req = await fetch(
      `/api/playlists/${id}/playlist_mp3s/${playlist_mp3_id}/move_higher`,
      {
        method: 'POST',
        mode: 'cors',
        body: JSON.stringify({}),
        headers: {
          'Content-Type': 'application/json',
        },
      }
    ).finally(() => {
      showWait(false)
    })

    const data = await req.json()
    setPlaylistMp3s(data.playlist_mp3s)
  }

  function moveHigher(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    const id = e.currentTarget.id
    doMoveHigher(id)
  }

  const doMoveLower = async (playlist_mp3_id: string) => {
    showWait(true)
    const req = await fetch(
      `/api/playlists/${id}/playlist_mp3s/${playlist_mp3_id}/move_lower`,
      {
        method: 'POST',
        mode: 'cors',
        body: JSON.stringify({}),
        headers: {
          'Content-Type': 'application/json',
        },
      }
    ).finally(() => {
      showWait(false)
    })

    const data = await req.json()
    setPlaylistMp3s(data.playlist_mp3s)
  }

  function moveLower(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    const id = e.currentTarget.id
    doMoveLower(id)
  }

  const doDeleteMp3 = async (playlist_mp3_id: string) => {
    showWait(true)
    const req = await fetch(
      `/api/playlists/${id}/playlist_mp3s/${playlist_mp3_id}`,
      {
        method: 'DELETE',
        mode: 'cors',
        body: JSON.stringify({}),
        headers: {
          'Content-Type': 'application/json',
        },
      }
    ).finally(() => {
      showWait(false)
    })

    const data = await req.json()
    setPlaylistMp3s(data.playlist_mp3s)
  }

  function deleteMp3(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    const id = e.currentTarget.id
    if (!id) return

    doDeleteMp3(id)
  }

  function searchByAlbumName(e: React.MouseEvent<HTMLAnchorElement>) {
    e.preventDefault()
    const album_name = e.currentTarget.dataset.albumName
    setQ(`album:"${album_name}"`)
    setShow({ entity: 'mp3s', id: '' })
  }

  function searchByArtistName(e: React.MouseEvent<HTMLAnchorElement>) {
    e.preventDefault()
    const artist_name = e.currentTarget.dataset.artistName
    setQ(`artist:"${artist_name}"`)
    setShow({ entity: 'mp3s', id: '' })
  }

  return (
    <div className='container-fluid pt-4 px-0'>
      <table className='table table-striped table-hover'>
        <thead>
          <tr>
            <th>Title</th>
            <th>Album</th>
            <th className='text-center'>Track</th>
            <th>Artist</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {playlistMp3s.map((playlistMp3) => (
            <tr
              className='align-middle'
              key={`playlist_mp3-${playlistMp3['id']}`}
            >
              <td>
                <a
                  href='#'
                  onClick={playMp3}
                  id={playlistMp3['mp3_id'].toString()}
                >
                  {playlistMp3['title']}
                </a>
              </td>
              <td>
                <a
                  href='#'
                  onClick={searchByAlbumName}
                  data-album-name={playlistMp3['album_name']}
                >
                  {playlistMp3['album_name']}
                </a>
              </td>
              <td className='text-center'>{playlistMp3['track']}</td>
              <td>
                <a
                  href='#'
                  onClick={searchByArtistName}
                  data-artist-name={playlistMp3['artist_name']}
                >
                  {playlistMp3['artist_name']}
                </a>
              </td>
              <td className='tight'>
                <div
                  className='btn-group'
                  role='group'
                  aria-label='Playlist MP3 actions'
                >
                  <div className='btn-group' role='group'>
                    <button
                      type='button'
                      className={`btn btn-sm btn-primary${
                        playlistMp3['first'] ? ' disabled' : ''
                      }`}
                      onClick={moveHigher}
                      id={playlistMp3['id'].toString()}
                    >
                      Up
                    </button>
                    <button
                      type='button'
                      className={`btn btn-sm btn-primary${
                        playlistMp3['last'] ? ' disabled' : ''
                      }`}
                      onClick={moveLower}
                      id={playlistMp3['id'].toString()}
                    >
                      Down
                    </button>
                    <button
                      type='button'
                      className='btn btn-sm btn-primary'
                      onClick={deleteMp3}
                      id={playlistMp3['id'].toString()}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
