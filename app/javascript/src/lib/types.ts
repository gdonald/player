export interface Mp3Type {
  [key: string]: string
  id: string
  title: string
  artist_name: string
  album_name: string
  track: string
}

export interface EditMp3Type {
  id: number
  title: string
  album_name: string
  artist_name: string
  track: number
}

export interface EditMp3ErrorsType {
  title: string
  album_name: string
  artist_name: string
  track: string
}

export interface NewPlaylistErrorType {
  name: string | null
}

export interface PlaylistMp3Type {
  id: number
  first: boolean
  last: boolean
  mp3: Mp3Type
}

export interface NewPlaylistType {
  name: string
}

export interface PlaylistType {
  id: number
  name: string
  mp3s_count: number
}

export interface QueuedMp3Type {
  id: number
  position: number
  mp3: Mp3Type
}

export interface SourceErrorsType {
  path: string
}

export interface ShowType {
  entity: string
  id: number
}

export interface SourceType {
  id: number
  path: string
  mp3s_count: number
}
