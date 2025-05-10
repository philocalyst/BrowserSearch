#!/usr/bin/env osascript -l JavaScript
function run(argv) {
  const browser = argv[0];
  const query = argv[1];
  
  const [windowIndex, tabIndex] = query.split(",").map(x => parseInt(x));
  
  // For Arc, we need to handle spaces - assuming space 0 by default
  // Original format was "windowIndex,spaceIndex,tabIndex"
  const spaceIndex = 0;
  
  const app = Application(browser);
  
  try {
    app.windows[windowIndex].spaces[spaceIndex].focus();
    app.windows[windowIndex].spaces[spaceIndex].tabs[tabIndex].select();
    app.activate();
    
    return JSON.stringify({
      status: "success",
      browser: browser,
      windowIndex: windowIndex,
      spaceIndex: spaceIndex,
      tabIndex: tabIndex
    });
  } catch (e) {
    return JSON.stringify({
      status: "error",
      message: e.toString()
    });
  }
}
