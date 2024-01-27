import React, { useEffect, useState } from 'react'
import { ShowType, QueuedMp3Type } from '../lib/types'
import Alert from './Alert'
import Mp3 from './Mp3'
import Menu from './Menu'
import Mp3s from './Mp3s'
import Audio from './Audio'
import Playlist from './Playlist'
import Playlists from './Playlists'
import Queue from './Queue'
import Source from './Source'
import Sources from './Sources'
import Wait from './Wait'
import LoginForm from './LoginForm'

export default function App() {
  const [authenticated, setAuthenticated] = useState<boolean>(false)
  const [q, setQ] = useState<string | null>(null)
  const [show, setShow] = useState<ShowType | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [wait, setWait] = useState<boolean>(false)
  const [src, setSrc] = useState<string | null>(null)
  const [srcEnded, setSrcEnded] = useState<boolean>(false)
  const [queuedMp3s, setQueuedMp3s] = useState<QueuedMp3Type[]>([])
  const [currentQueuedMp3, setCurrentQueuedMp3] =
    useState<QueuedMp3Type | null>(null)

  const getQueuedMp3s = async () => {
    const req = await fetch(`/api/queued_mp3s`)
    const data = await req.json()
    setQueuedMp3s(data.queued_mp3s)
  }

  const removeQueuedMp3 = async (id: string) => {
    const req = await fetch(`/api/queued_mp3s/${id}`, {
      method: 'DELETE',
      mode: 'cors',
      body: JSON.stringify({}),
      headers: {
        'Content-Type': 'application/json',
      },
    }).finally(() => {})

    const data = await req.json()
    const queuedMp3s = data.queued_mp3s
    setQueuedMp3s(queuedMp3s)

    if (data.message) {
      showMessage(data.message)
    }
  }

  function nextQueuedMp3() {
    if (!srcEnded) return
    doSrcEnded(false)

    if (!queuedMp3s || queuedMp3s.length === 0) {
      setCurrentQueuedMp3(null)
      setSrc(null)
      return
    }

    if (queuedMp3s.length === 1) {
      setCurrentQueuedMp3(null)
      setSrc(null)
      removeQueuedMp3(queuedMp3s[0]['id'].toString())
      return
    }

    if (queuedMp3s.length > 1) {
      setCurrentQueuedMp3(queuedMp3s[1])
      playSrc(`/api/mp3s/${queuedMp3s[1]['mp3']['id']}/play`)
      removeQueuedMp3(queuedMp3s[0]['id'].toString())
      return
    }
  }

  const playQueue = () => {
    if (!queuedMp3s || queuedMp3s.length === 0) return

    setCurrentQueuedMp3(queuedMp3s[0])
    playSrc(`/api/mp3s/${queuedMp3s[0]['mp3']['id']}/play`)
  }

  const createQueuedMp3sFromPlaylist = async (id: string) => {
    showWait(true)
    const req = await fetch(`/api/playlists/${id}/enqueue`, {
      method: 'POST',
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
    }

    const queuedMp3s = data.queued_mp3s
    setQueuedMp3s(queuedMp3s)

    if (data.queued_mp3s.length > 0 && !currentQueuedMp3) {
      setCurrentQueuedMp3(data.queued_mp3s[0])
      playSrc(`/api/mp3s/${data.queued_mp3s[0]['mp3']['id']}/play`)
    }
  }

  const createQueuedMp3 = async (mp3_id: string) => {
    showWait(true)
    const req = await fetch(`/api/queued_mp3s`, {
      method: 'POST',
      mode: 'cors',
      body: JSON.stringify({
        queued_mp3: {
          mp3_id: mp3_id,
        },
      }),
      headers: {
        'Content-Type': 'application/json',
      },
    }).finally(() => {
      showWait(false)
    })

    const data = await req.json()
    if (data.message) {
      showMessage(data.message)
    }

    const queuedMp3s = data.queued_mp3s
    setQueuedMp3s(queuedMp3s)

    if (data.queued_mp3s.length > 0 && !currentQueuedMp3) {
      setCurrentQueuedMp3(data.queued_mp3s[0])
      playSrc(`/api/mp3s/${data.queued_mp3s[0]['mp3']['id']}/play`)
    }
  }

  const doSrcEnded = (value: boolean) => {
    setSrcEnded(value)
  }

  const playSrc = (src: string) => {
    setSrc(src)
  }

  const showMessage = (message: string) => {
    setMessage(message)
  }

  const showWait = (val: boolean) => {
    setWait(val)
  }

  const checkAuthenticated = async () => {
    const req = await fetch(`/api/sessions/active`, {
      method: 'GET',
      mode: 'cors',
      credentials: 'include',
      headers: {
        'Content-Type': 'application/json',
      },
    }).then((res) => {
      if (res.ok) {
        setAuthenticated(true)
      } else {
        setAuthenticated(false)
      }
    })
  }

  useEffect(() => {
    setShow({ entity: 'mp3s', id: 0 })
  }, [])

  useEffect(() => {
    if (!authenticated) return

    getQueuedMp3s()
  }, [authenticated])

  useEffect(() => {
    nextQueuedMp3()
  }, [srcEnded])

  useEffect(() => {
    checkAuthenticated()
  }, [])

  if (!authenticated) {
    return <LoginForm setAuthenticated={setAuthenticated}></LoginForm>
  }

  if (authenticated) {
    return (
      <div className='container-fluid vh-100'>
        <div className='row'>
          <div className='col-2'>
            <div className='row border-bottom border-dark'>
              <div className='menu-items'>
                {<Wait wait={wait}></Wait>}
                <Menu
                  setShow={setShow}
                  showMessage={showMessage}
                  show={show}
                ></Menu>
              </div>
            </div>
            <div className='row'>
              <div className='queue-wrapper'>
                <div className='queue-content'>
                  <Queue
                    queuedMp3s={queuedMp3s}
                    currentQueuedMp3={currentQueuedMp3}
                    playQueue={playQueue}
                    removeQueuedMp3={removeQueuedMp3}
                    doSrcEnded={doSrcEnded}
                    setSrc={setSrc}
                  ></Queue>
                </div>
              </div>
            </div>
          </div>
          <div className='col-10 min-vh-100 border-start border-dark'>
            <div className='main-wrapper'>
              <div className='row main-content pt-2'>
                <div className='p-0'>
                  {message && (
                    <Alert message={message} setMessage={setMessage}></Alert>
                  )}
                  {show && show.entity === 'mp3s' && (
                    <Mp3s
                      showWait={showWait}
                      playSrc={playSrc}
                      showMessage={showMessage}
                      setShow={setShow}
                      createQueuedMp3={createQueuedMp3}
                      q={q}
                      setQ={setQ}
                    ></Mp3s>
                  )}
                  {show && show.entity === 'mp3' && (
                    <Mp3
                      id={show.id}
                      showWait={showWait}
                      showMessage={showMessage}
                      setShow={setShow}
                    ></Mp3>
                  )}
                  {show && show.entity === 'playlists' && (
                    <Playlists
                      setShow={setShow}
                      showMessage={showMessage}
                      showWait={showWait}
                      createQueuedMp3sFromPlaylist={
                        createQueuedMp3sFromPlaylist
                      }
                    ></Playlists>
                  )}
                  {show && show.entity === 'playlist' && (
                    <Playlist
                      setShow={setShow}
                      showMessage={showMessage}
                      showWait={showWait}
                      createQueuedMp3={createQueuedMp3}
                      id={show.id}
                      createQueuedMp3sFromPlaylist={
                        createQueuedMp3sFromPlaylist
                      }
                      setQ={setQ}
                    ></Playlist>
                  )}
                  {show && show.entity === 'sources' && (
                    <Sources
                      setShow={setShow}
                      showMessage={showMessage}
                      showWait={showWait}
                    ></Sources>
                  )}
                  {show && show.entity === 'source' && (
                    <Source
                      setShow={setShow}
                      showMessage={showMessage}
                      showWait={showWait}
                      id={show.id}
                    ></Source>
                  )}
                </div>
              </div>
            </div>
            <div className='row audio-content border-top border-dark'>
              <div className='text-center pt-3'>
                <Audio src={src} doSrcEnded={doSrcEnded}></Audio>
              </div>
            </div>
          </div>
        </div>
      </div>
    )
  }
}
