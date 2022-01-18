import {useEffect} from 'react'

export const useAsyncEffect = (fn, ...args) => {
  useEffect(() => {
    fn()
  }, ...args)
}
