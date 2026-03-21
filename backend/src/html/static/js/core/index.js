/**
 * Core utilities module
 * @module core
 * 
 * Public API for core DOM and network utilities.
 */

// DOM utilities
export { $, $$, on, dataSet, toArray } from './dom.js';

// Network utilities  
export { jsonFetch, postJson, patchJson, deleteJson, formToJSON } from './net.js';
