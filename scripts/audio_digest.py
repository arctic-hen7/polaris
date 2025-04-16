#!/usr/bin/env python
# Converts the digest string parsed in from stdin into an audio file using `gpt-4o-mini`
# to convert the digest into a script first, and then speaking it aloud using the `tts`
# API.

DIGEST_SCRIPT_INSTRUCTIONS = "You are a fun assistant in part of a pipeline to deliver a spoken daily digest to me, a founder and student. You will be given the raw Markdown of a daily digest file containing events, daily notes (i.e. things to remember), goals for the day, week, and general goals that are shown every day, and urgent actions that need to be done during the day (which might include problems/projects to be worked on). You should provide a script version of this that can be spoken fluently by a text-to-speech engine. Make sure to include all the detail of the daily digest and not change anything, just reformat it so it can be spoken fluently. You should open with a cheerful \"Good morning\" or similar, and close with a positive message to have a great day. Otherwise, don't add too many embellishments."
AUDIO_INSTRUCTIONS = "Speak in a cheerful and upbeat tone."
TTS_VOICE = "nova"

if __name__ == "__main__":
    import sys
    from openai import OpenAI

    audio_file_path = sys.argv[1] if len(sys.argv) > 1 else None
    if not audio_file_path:
        print("Please provide a path to output audio to.")
        sys.exit(1)

    digest_md = sys.stdin.read()
    client = OpenAI()

    print("Converting digest to script...")
    response = client.responses.create(
        model="gpt-4.1-nano",
        instructions=DIGEST_SCRIPT_INSTRUCTIONS,
        input=digest_md
    )

    print("Generating digest audio...")
    with client.audio.speech.with_streaming_response.create(
        model="tts-1",
        voice=TTS_VOICE,
        input=response.output_text,
        # instructions=AUDIO_INSTRUCTIONS,
    ) as response:
        response.stream_to_file(audio_file_path)
