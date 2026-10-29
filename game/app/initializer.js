const loader = document.getElementById("loader");
const status = document.getElementById("loader-status");
const progress = document.getElementById("loader-progress");
const play = document.getElementById("loader-play");

function fail(reason) {
  status.textContent = reason;
  progress.hidden = true;
  document.getElementById("loader-releases").hidden = false;
}

async function hasWebGpu() {
  try {
    return (await navigator.gpu?.requestAdapter()) != null;
  } catch {
    return false;
  }
}

// Stop here so the browser never downloads the wasm it can't run.
if (!(await hasWebGpu())) {
  fail("Your browser does not support WebGPU, which this game needs.");
  throw new Error("WebGPU is not available");
}

const megabytes = (bytes) => (bytes / 1e6).toFixed(1);

export default function initializer() {
  return {
    onProgress: ({ current, total }) => {
      if (total === 0) {
        status.textContent = `Loading (${megabytes(current)} MB)`;
        return;
      }
      progress.max = total;
      progress.value = current;
      status.textContent = current < total
        ? `Loading (${megabytes(current)} / ${megabytes(total)} MB)`
        : "Starting";
    },
    // The click on Play is the user gesture that browsers need to start audio.
    onSuccess: () => {
      status.hidden = true;
      progress.hidden = true;
      play.hidden = false;
      play.focus();
      play.addEventListener("click", () => {
        loader.remove();
        document.querySelector("canvas")?.focus();
      });
    },
    onFailure: (error) => {
      console.error(error);
      fail("The game failed to load.");
    },
  };
}
