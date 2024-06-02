# Watch Drift

This is a tool for managing and reviewing your watch collection's accuracy.

When setting my mechanical watch time, I typically will tune how far ahead I set it based on the known drift of the watch. I've used tools like [toolwatch.io](toolwatch.io), but I didn't want to have to go the web and click around every time I set the time on my watch. The cli interface for doing measures is also much nicer than the hand-rolled spreadsheet I was using.

I don't need atomic level accuracy, so using `chrono::DateTime` is enough for me. Your computer clock
will drift and affect your results. This is intended more for an order of magnitude, running fast/slow kind of thing.

To force your computer to timesync now and perhaps increase the accuracy (macOS):
```
sudo sntp -sS time.apple.com
```

This tool has a hardcoded path since I'm the only one using it. If someone comes along with a PR allowing for a custom path, I'll gladly take it. Otherwise, check it out and change that path.

## Commands
```
  new          Create a new watch
  ls           Lists watches in the database. Takes an optional regex pattern to filter
  start        Start a measure for the given watch
  end          End or Update a measure for the given watch
  recalculate  Force a recalculation of how the watch is running. Useful after manually editing the database file
  log          Mark down a wear of the given watch for today
```

