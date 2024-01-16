import React, { useEffect, useState } from 'react'
import { DebounceInput } from 'react-debounce-input'
import PlaylistsForm from './PlaylistsForm'
import { Mp3Type, PlaylistType } from '../lib/types'

export default function Mp3s({
  showWait,
  showMessage,
  setShow,
  createQueuedMp3,
  q,
  setQ,
}: {
  showWait: Function
  playSrc: Function
  showMessage: Function
  setShow: Function
  createQueuedMp3: Function
  q: string | null
  setQ: Function
}) {
  const [mp3s, setMp3s] = useState<Mp3Type[]>([])
  const [mp3Ids, setMp3Ids] = useState<string[]>([])
  const [playlists, setPlaylists] = useState<PlaylistType[]>([])
  const [sortBy, setSortBy] = useState<string | null>(null)

  const getMp3s = async () => {
    showWait(true)
    const sortQuery = sortBy ? `sort=${sortBy}` : ''
    await fetch(`/api/mp3s?${sortQuery}`)
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        setMp3s(data.mp3s)
      })
      .finally(() => {
        showWait(false)
      })
  }

  const searchMp3s = async (url: string) => {
    showWait(true)
    const sortQuery = sortBy ? `sort=${sortBy}` : ''
    await fetch(`${url}&${sortQuery}`)
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        setMp3s(data.mp3s)
      })
      .finally(() => {
        showWait(false)
      })
  }

  const getPlaylists = async () => {
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
  }

  const getPlaylistsAndMp3s = async () => {
    showWait(true)

    let requestsCompleted = 0
    const sortQuery = sortBy ? `sort=${sortBy}` : ''
    const mp3sUrl = q
      ? `/api/mp3s/search?q=${q}&${sortQuery}`
      : `/api/mp3s?${sortQuery}`

    await fetch(mp3sUrl)
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        setMp3s(data.mp3s)
      })
      .finally(() => {
        if (++requestsCompleted === 2) {
          showWait(false)
        }
      })

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
        if (++requestsCompleted === 2) {
          showWait(false)
        }
      })
  }

  const doUpdatePlaylist = async (playlistId: string, mp3Id: string) => {
    showWait(true)
    await fetch(`/api/playlists/${playlistId}`, {
      method: 'PUT',
      mode: 'cors',
      body: JSON.stringify({
        playlist: {
          playlist_mp3s_attributes: [{ mp3_id: mp3Id }],
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

  function search(e: React.ChangeEvent<HTMLInputElement>) {
    e.preventDefault()
    const q = e.target.value
    setQ(q)
    searchMp3s(`/api/mp3s/search?q=${q}`)
  }

  function searchArtist(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    const q = `artist:"${e.currentTarget.innerText}"`
    setQ(q)
    searchMp3s(`/api/mp3s/search?q=${q}`)
  }

  function searchAlbum(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    const q = `album:"${e.currentTarget.innerText}"`
    setQ(q)
    searchMp3s(`/api/mp3s/search?q=${q}`)
  }

  function clearSearch(e: React.MouseEvent<HTMLButtonElement, MouseEvent>) {
    e.preventDefault()
    setQ(null)
    getMp3s()
  }

  function playMp3(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    createQueuedMp3(e.currentTarget.id)
  }

  function selectMp3(e: React.MouseEvent<HTMLInputElement, MouseEvent>) {
    const tbody = e.currentTarget.closest('tbody')
    if (!tbody) return

    const checkboxes = tbody.querySelectorAll('input[type="checkbox"]')
    const checkedCheckboxes = Array.from(checkboxes).filter(
      (checkbox) => (checkbox as HTMLInputElement).checked
    )

    const ids = checkedCheckboxes.map((checkbox) => checkbox.id)
    setMp3Ids(ids)

    const table = e.currentTarget.closest('table')
    if (!table) return

    let selectAllCheckbox: HTMLInputElement | null = table.querySelector(
      'thead input[type="checkbox"]'
    )

    if (!selectAllCheckbox) return

    selectAllCheckbox.checked = checkedCheckboxes.length === checkboxes.length
  }

  function selectAllNoneMp3s(
    e: React.MouseEvent<HTMLInputElement, MouseEvent>
  ) {
    const checked = e.currentTarget.checked
    const table = e.currentTarget.closest('table')
    if (!table) return

    const tbody = table.querySelector('tbody')
    if (!tbody) return

    const checkboxes = tbody.querySelectorAll('input[type="checkbox"]')
    checkboxes.forEach((checkbox) => {
      ;(checkbox as HTMLInputElement).checked = checked
    })

    if (checked) {
      const ids = Array.from(checkboxes).map((checkbox) => checkbox.id)
      setMp3Ids(ids)
    } else {
      setMp3Ids([])
    }
  }

  function editMp3(e: React.MouseEvent<HTMLButtonElement, MouseEvent>) {
    e.preventDefault()
    setShow({ entity: 'mp3', id: e.currentTarget.id })
  }

  function addToPlaylist(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    const playlistId = e.currentTarget.dataset.playlistId
    const mp3Id = e.currentTarget.dataset.mp3Id
    if (!playlistId || !mp3Id) return

    doUpdatePlaylist(playlistId, mp3Id)
  }

  function sortByAlbum(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    sortBy === 'album_asc' ? setSortBy('album_desc') : setSortBy('album_asc')
  }

  function sortByArtist(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    sortBy === 'artist_asc' ? setSortBy('artist_desc') : setSortBy('artist_asc')
  }

  function sortByTrack(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    sortBy === 'track_asc' ? setSortBy('track_desc') : setSortBy('track_asc')
  }

  function sortByTitle(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    sortBy === 'title_asc' ? setSortBy('title_desc') : setSortBy('title_asc')
  }

  useEffect(() => {
    if (q) {
      getPlaylists()
      searchMp3s(`/api/mp3s/search?q=${q}`)
    } else {
      getPlaylistsAndMp3s()
    }
  }, [])

  useEffect(() => {
    if (q) {
      getPlaylists()
      searchMp3s(`/api/mp3s/search?q=${q}`)
    } else {
      getMp3s()
    }
  }, [sortBy])

  return (
    <div className='container-fluid'>
      <nav aria-label='breadcrumb'>
        <ol className='breadcrumb'>
          <li className='breadcrumb-item' aria-current='page'>
            <b>
              <i className='bi-music-note-beamed'></i> MP3s
            </b>
          </li>
        </ol>
      </nav>
      <div className='search'>
        <form onSubmit={(e) => e.preventDefault()} className='px-0'>
          <div className='input-group mb-0'>
            <DebounceInput
              className='form-control'
              debounceTimeout={500}
              onChange={search}
              value={q || ''}
              placeholder='Search'
            />
            <button
              className='btn btn-primary'
              type='button'
              id='button-addon-search'
              onClick={clearSearch}
            >
              Clear Search
            </button>
          </div>
        </form>
      </div>
      {mp3Ids.length > 0 && (
        <PlaylistsForm
          showWait={showWait}
          mp3Ids={mp3Ids}
          showMessage={showMessage}
          setShow={setShow}
          playlists={playlists}
          q={q}
        ></PlaylistsForm>
      )}
      <div className='list mt-2'>
        {mp3s.length === 0 && (
          <p className='text-center pt-5'>No mp3s found.</p>
        )}

        {mp3s.length > 0 && (
          <table className='table table-striped table-hover'>
            <thead>
              <tr>
                <th className='tight'>
                  {mp3s.length > 0 && (
                    <input type='checkbox' onClick={selectAllNoneMp3s} />
                  )}
                </th>
                <th>
                  <a href='#' onMouseDown={sortByTitle}>
                    Title
                  </a>
                </th>
                <th>
                  <a href='#' onMouseDown={sortByAlbum}>
                    Album
                  </a>
                </th>
                <th className='text-center'>
                  <a href='#' onMouseDown={sortByTrack}>
                    Track
                  </a>
                </th>
                <th>
                  <a href='#' onMouseDown={sortByArtist}>
                    Artist
                  </a>
                </th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {mp3s.map((mp3) => (
                <tr className='align-middle' key={`mp3-${mp3['id']}`}>
                  <td>
                    <input
                      type='checkbox'
                      onClick={selectMp3}
                      id={mp3['id'].toString()}
                    />
                  </td>
                  <td>
                    <a href='#' onClick={playMp3} id={mp3['id'].toString()}>
                      {mp3['title']}
                    </a>
                  </td>
                  <td>
                    <a href='#' onClick={searchAlbum}>
                      {mp3['album_name']}
                    </a>
                  </td>
                  <td className='text-center'>{mp3['track']}</td>
                  <td>
                    <a href='#' onClick={searchArtist}>
                      {mp3['artist_name']}
                    </a>
                  </td>
                  <td className='tight'>
                    <div
                      className='btn-group'
                      role='group'
                      aria-label='Mp3 actions'
                    >
                      {playlists.length > 0 && (
                        <div className='btn-group' role='group'>
                          <button
                            type='button'
                            className='btn btn-sm btn-primary dropdown-toggle'
                            data-bs-toggle='dropdown'
                            aria-expanded='false'
                          >
                            Add to Playlist
                          </button>
                          <ul
                            className='dropdown-menu'
                            key={`mp3-pl-options-${mp3['id']}`}
                          >
                            {playlists.map((playlist) => (
                              <li key={`pl-option-${playlist['id']}`}>
                                <a
                                  className='dropdown-item'
                                  href='#'
                                  data-playlist-id={playlist['id']}
                                  data-mp3-id={mp3['id']}
                                  onClick={addToPlaylist}
                                >
                                  {playlist['name']}
                                </a>
                              </li>
                            ))}
                          </ul>
                        </div>
                      )}
                      <button
                        type='button'
                        className='btn btn-sm btn-primary'
                        id={mp3['id'].toString()}
                        onClick={editMp3}
                      >
                        Edit
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
