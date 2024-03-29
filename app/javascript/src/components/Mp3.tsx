import React, { useState, useEffect } from 'react'
import { EditMp3Type, EditMp3ErrorsType } from '../lib/types'

export default function Mp3({
  id,
  showWait,
  showMessage,
  setShow,
}: {
  id: number
  showWait: Function
  showMessage: Function
  setShow: Function
}) {
  const [mp3, setMp3] = useState<EditMp3Type | null>(null)
  const [errors, setErrors] = useState<EditMp3ErrorsType | null>(null)

  const getMp3 = async () => {
    showWait(true)
    await fetch(`/api/mp3s/${id}`)
      .then((res) => {
        if (!res.ok) {
          window.location.href = '/'
        }

        return res.json()
      })
      .then((data) => {
        setMp3(data.mp3)
      })
      .finally(() => {
        showWait(false)
      })
  }

  const save = async () => {
    if (!mp3) return

    showWait(true)
    await fetch(`/api/mp3s/${id}`, {
      method: 'PUT',
      mode: 'cors',
      body: JSON.stringify({
        mp3: {
          title: mp3.title,
          artist: mp3.artist_name,
          album: mp3.album_name,
          track: mp3.track,
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
          setMp3(data.mp3)
          setErrors(null)
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

  function saveMp3(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    save()
  }

  function showMp3s(
    e: React.MouseEvent<HTMLAnchorElement> | React.MouseEvent<HTMLButtonElement>
  ) {
    e.preventDefault()
    showMessage('')
    setShow({ entity: 'mp3s', id: 0 })
  }

  useEffect(() => {
    getMp3()
  }, [id])

  if (!mp3) return <></>

  return (
    <div className='container-fluid'>
      <nav aria-label='breadcrumb'>
        <ol className='breadcrumb'>
          <li className='breadcrumb-item'>
            <a href='#' onClick={showMp3s}>
              Mp3s
            </a>
          </li>
          <li className='breadcrumb-item' aria-current='page'>
            {mp3['title']}
          </li>
        </ol>
      </nav>

      <form className='form'>
        <div className='mb-3'>
          <label htmlFor='title' className='form-label'>
            Title
          </label>
          <input
            id='title'
            type='text'
            className='form-control'
            name='mp3[title]'
            defaultValue={mp3['title']}
            onChange={(e) => {
              setMp3({ ...mp3, title: e.target.value })
            }}
            onBlur={(e) => {
              setMp3({ ...mp3, title: e.target.value })
            }}
          />
          {errors && <div className='error'>{errors['title']}</div>}
        </div>

        <div className='mb-3'>
          <label htmlFor='artist' className='form-label'>
            Artist
          </label>
          <input
            id='artist_name'
            type='text'
            className='form-control'
            name='mp3[artist_name]'
            defaultValue={mp3['artist_name']}
            onChange={(e) => {
              setMp3({ ...mp3, artist_name: e.target.value })
            }}
            onBlur={(e) => {
              setMp3({ ...mp3, artist_name: e.target.value })
            }}
          />
          {errors && <div className='error'>{errors['artist_name']}</div>}
        </div>

        <div className='mb-3'>
          <label htmlFor='album' className='form-label'>
            Album
          </label>
          <input
            id='album_name'
            type='text'
            className='form-control'
            name='mp3[album_name]'
            defaultValue={mp3['album_name']}
            onChange={(e) => {
              setMp3({ ...mp3, album_name: e.target.value })
            }}
            onBlur={(e) => {
              setMp3({ ...mp3, album_name: e.target.value })
            }}
          />
          {errors && <div className='error'>{errors['album_name']}</div>}
        </div>

        <div className='mb-3'>
          <label htmlFor='track' className='form-label'>
            Track
          </label>
          <input
            id='track'
            type='text'
            className='form-control'
            name='mp3[track]'
            defaultValue={mp3['track']}
            onChange={(e) => {
              setMp3({ ...mp3, track: parseInt(e.target.value) })
            }}
            onBlur={(e) => {
              setMp3({ ...mp3, track: parseInt(e.target.value) })
            }}
          />
          {errors && <div className='error'>{errors['track']}</div>}
        </div>

        <div className='btn-group' role='group' aria-label='Save MP3'>
          <button type='submit' className='btn btn-primary' onClick={saveMp3}>
            Save
          </button>
          <button type='submit' className='btn btn-primary' onClick={showMp3s}>
            Go Back
          </button>
        </div>
      </form>
    </div>
  )
}
