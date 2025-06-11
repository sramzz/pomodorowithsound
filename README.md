# Pomodoro with Sound
A web page that allows you to track your Pomodoros
# Currently
- The static webpage allows you to:
  - Add a goal
  - Set up the time of the pomodoro (between 5 to 90 minutes in a hardcoded dropdown)
  - Start, pause, reset, end the pomodoro
  - Show a notification when each pomodoro ends
  - Make a sound when each pomodoro ends
  - Show in the log each session
  - Test the sound

# Next Steps
## Near term
  - Host this in my website
  - Add caching to keep log in the browser even if you close the tab
  - Create a Tauri version to run a OS agnostic local app
  - Add a local db for the Tauri App to keep the log
## Long term:
  - An app that allows you to:
    - Fill in goals
    - Fill in the micro tasks of each goal,
    - Add estimated duration time per micro task,
    - Add Deadline of the goal and each micro task,
    - Rank the goals and micro tasks by priority,
    - Decide how many pomodoros of X minutes will each task take
    - Add the resting time after pomodoro,
    - Connect to Google or Outlook Calendar.
    - Decide from when until when you want to do each task (in date and time)
    - Create those tasks in Google, Apple, and Outlook calendars.
    - Sync the tasks and meetings from the app and the integrated calendars. (To review how to implement it)
    - A day view to see what you have for the day
    - A start day button
      - A day is composed of all micro tasks planned for that specific day
      - A day shows you all the tasks including the meetings
      - Once you click on start day
        - A sound is emited and you can see all the time which task you should be doing and the timer
        - A notification shows up when there are only 5 min left advising to wrap up
        - A sound is emited when the pomodoro ends and it shows which one is the next task
        - The rest time starts
        - A notifiaction 1 minute before the rest time ends shows up
        - A sound is emited when the rest time finishes which marks the start of the new task
        - Once your day is over it gives you a notification
    - Do all the above via API
    - Build an MCP that allows any model to handle the app
    - *FINAL GOAL:* you tell an LLM your goals, and it creates them in the app for you. Then you just need to click on start your day and you know what to do!
