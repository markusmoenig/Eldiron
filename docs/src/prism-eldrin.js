// Registers Eldrin syntax highlighting with Prism for Docusaurus code blocks.
// Exported as a function so it can be invoked from prism-include-languages
// on both the server and client.

const identifier = /[A-Za-z_]\w*/.source;

export default function loadEldrin(Prism) {
  Prism.languages.eldrin = {
    comment: {
      pattern: /\/\/.*/,
      greedy: true,
    },
    string: {
      pattern: /"(?:\\.|[^"\\])*"/,
      greedy: true,
    },
    number: [
      {
        // float
        pattern: /\b\d+\.\d+\b/,
        alias: "float",
      },
      {
        // integer
        pattern: /\b\d+\b/,
        alias: "integer",
      },
    ],
    boolean: {
      pattern: /\b(?:true|false)\b/,
      alias: "constant",
    },
    keyword:
      /\b(?:let|fn|if|else|for|match|return|while|break|import|in|out|inout|void|const|struct)\b/,
    operator: [
      {
        pattern: /\b(?:and|or)\b/,
        alias: "keyword",
      },
      {
        pattern: /==|!=|<=|>=|<|>|&&|\|\||\+|-|\*|\/|%|=|\?|\:|\./,
      },
    ],
    type: {
      pattern: /\b(?:vec[234]|float[234]?|int[234]?|mat[234])\b/,
      alias: "class-name",
    },
    function: {
      pattern: RegExp(`\\b${identifier}(?=\\s*\\()`),
    },
    punctuation: /[()[\]{},;]/,
    variable: {
      pattern: RegExp(`\\b${identifier}\\b`),
    },
  };

  // Alias used by the source scope in the provided grammar.
  Prism.languages.rusterix = Prism.languages.eldrin;
}
