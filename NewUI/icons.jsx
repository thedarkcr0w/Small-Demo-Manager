// SVG icon set — single-color line icons, currentColor stroke
const Icon = ({ d, size = 16, fill = false, stroke = 1.6, style }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill={fill ? 'currentColor' : 'none'}
       stroke="currentColor" strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round" style={style}>
    {d}
  </svg>
);

const IconLibrary  = (p) => <Icon {...p} d={<><path d="M3 5h13M3 10h13M3 15h13M3 20h13"/><path d="M19 5l2 2v13l-2 -2"/></>} />;
const IconRecent   = (p) => <Icon {...p} d={<><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></>} />;
const IconFavorite = (p) => <Icon {...p} d={<path d="M12 4l2.5 5.5L20 10.5l-4 4 1 6-5-3-5 3 1-6-4-4 5.5-1z"/>} />;
const IconTag      = (p) => <Icon {...p} d={<><path d="M4 4h7l9 9-7 7-9-9z"/><circle cx="8.5" cy="8.5" r="1.2" fill="currentColor"/></>} />;
const IconFolder   = (p) => <Icon {...p} d={<path d="M3 6.5a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>} />;
const IconSettings = (p) => <Icon {...p} d={<><circle cx="12" cy="12" r="2.6"/><path d="M19.4 14a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V20a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H4a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H10a1.7 1.7 0 0 0 1-1.5V4a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V10a1.7 1.7 0 0 0 1.5 1H20a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></>} />;
const IconSearch   = (p) => <Icon {...p} d={<><circle cx="11" cy="11" r="6.5"/><path d="M20 20l-3.5-3.5"/></>} />;
const IconPlay     = (p) => <Icon {...p} d={<path d="M7 5l12 7-12 7z"/>} fill />;
const IconReveal   = (p) => <Icon {...p} d={<><path d="M3 7h6l2 2h10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M9 14h6M12 11v6"/></>} />;
const IconRename   = (p) => <Icon {...p} d={<><path d="M4 20h4l11-11-4-4L4 16z"/><path d="M14 5l4 4"/></>} />;
const IconMove     = (p) => <Icon {...p} d={<><path d="M9 6l6 6-6 6"/><path d="M3 12h12"/></>} />;
const IconArchive  = (p) => <Icon {...p} d={<><rect x="3" y="4" width="18" height="4" rx="1"/><path d="M5 8v11a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8"/><path d="M10 12h4"/></>} />;
const IconTrash    = (p) => <Icon {...p} d={<><path d="M4 7h16"/><path d="M9 7V4h6v3"/><path d="M6 7l1 13a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1l1-13"/><path d="M10 11v6M14 11v6"/></>} />;
const IconCopy     = (p) => <Icon {...p} d={<><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/></>} />;
const IconUser     = (p) => <Icon {...p} d={<><circle cx="12" cy="8" r="3.6"/><path d="M4 21a8 8 0 0 1 16 0"/></>} />;
const IconFlag     = (p) => <Icon {...p} d={<><path d="M5 21V4"/><path d="M5 4h11l-2 4 2 4H5"/></>} />;
const IconChevron  = (p) => <Icon {...p} d={<path d="M9 6l6 6-6 6"/>} />;
const IconChevronD = (p) => <Icon {...p} d={<path d="M6 9l6 6 6-6"/>} />;
const IconClose    = (p) => <Icon {...p} d={<><path d="M6 6l12 12M18 6L6 18"/></>} />;
const IconCheck    = (p) => <Icon {...p} d={<path d="M5 12l5 5L20 6"/>} />;
const IconPlus     = (p) => <Icon {...p} d={<><path d="M12 5v14M5 12h14"/></>} />;
const IconFilter   = (p) => <Icon {...p} d={<path d="M3 5h18l-7 9v6l-4-2v-4z"/>} />;
const IconRows     = (p) => <Icon {...p} d={<><rect x="3" y="4" width="18" height="4" rx="1"/><rect x="3" y="10" width="18" height="4" rx="1"/><rect x="3" y="16" width="18" height="4" rx="1"/></>} />;
const IconGrid     = (p) => <Icon {...p} d={<><rect x="3" y="3" width="8" height="8" rx="1"/><rect x="13" y="3" width="8" height="8" rx="1"/><rect x="3" y="13" width="8" height="8" rx="1"/><rect x="13" y="13" width="8" height="8" rx="1"/></>} />;
const IconSplit    = (p) => <Icon {...p} d={<><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M14 4v16"/></>} />;
const IconSort     = (p) => <Icon {...p} d={<><path d="M7 4v16M7 4l-3 3M7 4l3 3"/><path d="M17 20V4M17 20l-3-3M17 20l3-3"/></>} />;
const IconRefresh  = (p) => <Icon {...p} d={<><path d="M4 4v6h6"/><path d="M20 20v-6h-6"/><path d="M5 14a8 8 0 0 0 14 3"/><path d="M19 10a8 8 0 0 0-14-3"/></>} />;
const IconMinus    = (p) => <Icon {...p} d={<path d="M5 12h14"/>} />;
const IconMax      = (p) => <Icon {...p} d={<rect x="5" y="5" width="14" height="14" rx="1"/>} />;
const IconCS       = (p) => <Icon {...p} stroke={1.4} d={<><path d="M4 7l8-3 8 3v8l-8 4-8-4z"/><path d="M12 4v16"/><path d="M4 7l8 4 8-4"/></>} />;
const IconScoreboard = (p) => <Icon {...p} d={<><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 9h18M8 13h2M8 16h2M14 13h2M14 16h2"/></>} />;
const IconHeadshot = (p) => <Icon {...p} d={<><circle cx="12" cy="9" r="4"/><path d="M5 21c0-3 3-5 7-5s7 2 7 5"/><path d="M12 5v1M12 12v1"/></>} />;
const IconExternal = (p) => <Icon {...p} d={<><path d="M14 4h6v6"/><path d="M20 4l-9 9"/><path d="M18 13v5a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h5"/></>} />;
const IconNote     = (p) => <Icon {...p} d={<><path d="M5 4h10l4 4v12a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z"/><path d="M14 4v5h5"/><path d="M8 13h7M8 17h5"/></>} />;
const IconClock    = (p) => <Icon {...p} d={<><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></>} />;
const IconUsers    = (p) => <Icon {...p} d={<><circle cx="9" cy="8" r="3.2"/><circle cx="17" cy="9" r="2.6"/><path d="M3 20a6 6 0 0 1 12 0"/><path d="M14 20a5 5 0 0 1 8-4"/></>} />;
const IconBomb     = (p) => <Icon {...p} d={<><circle cx="11" cy="14" r="6.5"/><path d="M16 9l3-3"/><path d="M17 5h3v3"/></>} />;
const IconServer   = (p) => <Icon {...p} d={<><rect x="3" y="4" width="18" height="6" rx="1.5"/><rect x="3" y="14" width="18" height="6" rx="1.5"/><path d="M7 7h.01M7 17h.01"/></>} />;
const IconMic      = (p) => <Icon {...p} d={<><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0"/><path d="M12 18v3"/></>} />;
const IconDownload = (p) => <Icon {...p} d={<><path d="M12 3v13"/><path d="M7 11l5 5 5-5"/><path d="M4 21h16"/></>} />;
const IconWave     = (p) => <Icon {...p} d={<><path d="M4 12h2M8 8v8M11 5v14M14 9v6M17 7v10M20 11v2"/></>} />;
const IconCollapse = (p) => <Icon {...p} d={<><path d="M9 5l-6 7 6 7"/><path d="M15 5h6v14h-6"/></>} />;
const IconExpand   = (p) => <Icon {...p} d={<><path d="M15 5l6 7-6 7"/><path d="M9 5H3v14h6"/></>} />;
const IconGithub   = (p) => <Icon {...p} stroke={1.6} d={<path d="M12 2a10 10 0 0 0-3.16 19.49c.5.09.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.9 1.53 2.34 1.09 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.94 0-1.1.39-1.99 1.03-2.69-.1-.25-.45-1.27.1-2.65 0 0 .84-.27 2.75 1.02a9.5 9.5 0 0 1 5 0c1.91-1.29 2.75-1.02 2.75-1.02.55 1.38.2 2.4.1 2.65.64.7 1.03 1.59 1.03 2.69 0 3.84-2.34 4.69-4.57 4.93.36.31.68.92.68 1.85v2.75c0 .26.18.58.69.48A10 10 0 0 0 12 2z"/>} />;
const IconCoffee   = (p) => <Icon {...p} d={<><path d="M4 8h13v6a5 5 0 0 1-5 5H9a5 5 0 0 1-5-5z"/><path d="M17 10h2a2.5 2.5 0 0 1 0 5h-2"/><path d="M8 3c0 1.5 1 2 1 3M12 3c0 1.5 1 2 1 3"/></>} />;

Object.assign(window, {
  Icon, IconLibrary, IconRecent, IconFavorite, IconTag, IconFolder, IconSettings,
  IconSearch, IconPlay, IconReveal, IconRename, IconMove, IconArchive, IconTrash,
  IconCopy, IconUser, IconFlag, IconChevron, IconChevronD, IconClose, IconCheck,
  IconPlus, IconFilter, IconRows, IconGrid, IconSplit, IconSort, IconRefresh,
  IconMinus, IconMax, IconCS, IconScoreboard, IconHeadshot, IconExternal, IconNote,
  IconClock, IconUsers, IconBomb, IconServer, IconMic, IconDownload, IconWave,
  IconCollapse, IconExpand, IconGithub, IconCoffee,
});
