import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const native = require("./fastsse.node");

export const Decoder = native.Decoder;
export const encodeComment = native.encodeComment;
export const encodeEvent = native.encodeEvent;
export const encodeRetry = native.encodeRetry;

export default native;
