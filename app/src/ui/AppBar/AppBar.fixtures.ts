import { displayPageTabs, fullPageTabs } from '../ChosenTabs/ChosenTabs.fixtures'

/** The Stage board's app bar: Stage chosen, the Launchkey connected, the audio calm, 1392 wide. */
export const boardAppBar = {
  displayTabs: displayPageTabs,
  fullTabs: fullPageTabs,
  chosen: 'stage',
  launchkey: true,
  failedPart: null,
  synthOn: true,
  dropouts: 0,
  bufferFrames: 256,
  cpu: 0.2,
  width: 1392,
}
