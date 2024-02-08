import React, { useEffect, useState } from 'react'
import { PlaylistMp3Type } from '../lib/types'

export default function PlaylistMp3s({
  id,
  showWait,
  createQueuedMp3,
  setQ,
  setShow,
  setMp3sCount,
}: {
  id: number
  showWait: Function
  createQueuedMp3: Function
  setQ: Function
  setShow: Function
  setMp3sCount: Function
}) {
  const [playlistMp3s, setPlaylistMp3s] = useState<PlaylistMp3Type[]>([])

  const getMp3s = async () => {
    showWait(true)
    await fetch(`/api/playlists/${id}/playlist_mp3s`)
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        setPlaylistMp3s(data.playlist_mp3s)
        setMp3sCount(data.playlist_mp3s.length)
      })
      .finally(() => {
        showWait(false)
      })
  }

  const doMoveHigher = async (playlist_mp3_id: string) => {
    showWait(true)
    await fetch(
      `/api/playlists/${id}/playlist_mp3s/${playlist_mp3_id}/move_higher`,
      {
        method: 'POST',
        mode: 'cors',
        body: JSON.stringify({}),
        headers: {
          'Content-Type': 'application/json',
        },
      }
    )
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        setPlaylistMp3s(data.playlist_mp3s)
      })
      .catch((err) => {
        console.log(err)
      })
      .finally(() => {
        showWait(false)
      })
  }

  const doMoveLower = async (playlist_mp3_id: string) => {
    showWait(true)
    await fetch(
      `/api/playlists/${id}/playlist_mp3s/${playlist_mp3_id}/move_lower`,
      {
        method: 'POST',
        mode: 'cors',
        body: JSON.stringify({}),
        headers: {
          'Content-Type': 'application/json',
        },
      }
    )
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        setPlaylistMp3s(data.playlist_mp3s)
      })
      .catch((err) => {
        console.log(err)
      })
      .finally(() => {
        showWait(false)
      })
  }

  const doDeleteMp3 = async (playlist_mp3_id: string) => {
    showWait(true)
    await fetch(`/api/playlists/${id}/playlist_mp3s/${playlist_mp3_id}`, {
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
        setPlaylistMp3s(data.playlist_mp3s)
        setMp3sCount(data.playlist_mp3s.length)
      })
      .catch((err) => {
        console.log(err)
      })
      .finally(() => {
        showWait(false)
      })
  }

  function playMp3(e: React.MouseEvent<HTMLAnchorElement>) {
    e.preventDefault()
    createQueuedMp3(e.currentTarget.id)
  }

  function moveHigher(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    doMoveHigher(e.currentTarget.id)
  }

  function moveLower(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    doMoveLower(e.currentTarget.id)
  }

  function deleteMp3(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    doDeleteMp3(e.currentTarget.id)
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

  useEffect(() => {
    getMp3s()
  }, [])

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
                  id={playlistMp3['mp3']['id'].toString()}
                >
                  {playlistMp3['mp3']['title']}
                </a>
              </td>
              <td>
                <a
                  href='#'
                  onClick={searchByAlbumName}
                  data-album-name={playlistMp3['mp3']['album_name']}
                >
                  {playlistMp3['mp3']['album_name']}
                </a>
              </td>
              <td className='text-center'>{playlistMp3['mp3']['track']}</td>
              <td>
                <a
                  href='#'
                  onClick={searchByArtistName}
                  data-artist-name={playlistMp3['mp3']['artist_name']}
                >
                  {playlistMp3['mp3']['artist_name']}
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
