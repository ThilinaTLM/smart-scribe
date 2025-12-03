import { GoogleGenAI, type GenerateContentParameters } from "@google/genai";

const AUDIO_FILE_PATH = "/home/tlm/Downloads/output.mp3";

async function main() {
  const ai = new GoogleGenAI({
    apiKey: process.env.GEMINI_API_KEY,
  });

  const model = "gemini-flash-lite-latest";

  const file = Bun.file(AUDIO_FILE_PATH);
  const arrbuf = await file.arrayBuffer();
  const buffer = Buffer.from(arrbuf);
  const base64AudioFile = buffer.toString("base64");

  const contents = [
    {
      role: "user",
      parts: [
        {
          inlineData: {
            mimeType: "audio/mp3",
            data: base64AudioFile,
          },
        },
      ],
    },
  ];

  const config: GenerateContentParameters["config"] = {
    thinkingConfig: {
      thinkingBudget: 0,
    },
    systemInstruction:
      "You are an voice to text assistant who converts the given audio file into grammatically correct, context aware text output.",
  };

  const response = await ai.models.generateContent({
    model,
    config,
    contents,
  });
  console.log(response.text);
}

await main();
