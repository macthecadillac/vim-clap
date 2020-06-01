" Author: liuchengxu <xuliuchengxlc@gmail.com>
" Description: Vim client for the daemon job.

let s:save_cpo = &cpoptions
set cpoptions&vim

let s:req_id = get(s:, 'req_id', 0)
" Note: must use v:true/v:false for json_encode
let s:enable_icon = g:clap_enable_icon ? v:true : v:false

function! clap#client#send_request_initialize_global_env() abort
  let s:req_id += 1
  call clap#job#daemon#send_message(json_encode({
        \ 'id': s:req_id,
        \ 'method': 'initialize_global_env',
        \ 'params': {
        \   'is_nvim': has('nvim') ? v:true : v:false,
        \   'enable_icon': s:enable_icon,
        \   'clap_preview_size': g:clap_preview_size,
        \ }
        \ }))
endfunction

function! clap#client#send_request_on_init(params) abort
  let s:req_id += 1
  call clap#job#daemon#send_message(json_encode({
        \ 'id': s:req_id,
        \ 'method': 'client.on_init',
        \ 'params': a:params
        \ }))
endfunction

function! s:provider_source_cmd() abort
  if g:clap.provider.source_type == g:__t_string
    return g:clap.provider._().source
  elseif g:clap.provider.source_type == g:__t_func_string
    return g:clap.provider._().source()
  else
    return ''
  endif
endfunction

function! clap#client#send_request_on_init_default_impl() abort
  let s:req_id += 1
  call clap#job#daemon#send_message(json_encode({
        \ 'id': s:req_id,
        \ 'method': 'client.on_init',
        \ 'params': {
        \   'cwd': g:clap.provider.id ==# 'filer' ? clap#provider#filer#current_dir() : clap#rooter#working_dir(),
        \   'provider_id': g:clap.provider.id,
        \   'source_cmd': s:provider_source_cmd(),
        \ },
        \ }))
endfunction

function! clap#client#send_request_on_init_inner() abort
  let s:req_id += 1
  call clap#job#daemon#send_message(json_encode({
        \ 'id': s:req_id,
        \ 'method': 'client.on_init',
        \ 'params': {
        \   'cwd': g:clap.provider.id ==# 'filer' ? clap#provider#filer#current_dir() : clap#rooter#working_dir(),
        \   'provider_id': g:clap.provider.id,
        \ },
        \ }))
endfunction


function! clap#client#send_request_on_typed(params) abort
  let s:req_id += 1
  call clap#job#daemon#send_message(json_encode({
        \ 'id': s:req_id,
        \ 'method': 'client.on_typed',
        \ 'params': a:params
        \ }))
endfunction

function! clap#client#send_request_on_move() abort
  let s:req_id += 1
  let curline = g:clap.display.getcurline()
  let msg = {
      \ 'id': s:req_id,
      \ 'method': 'client.on_move',
      \ 'params': {
      \   'cwd': g:clap.provider.id ==# 'filer' ? clap#provider#filer#current_dir() : clap#rooter#working_dir(),
      \   'curline': curline,
      \   'provider_id': g:clap.provider.id,
      \ },
      \ }
  if g:clap.provider.id ==# 'tags' || g:clap.provider.id ==# 'blines'
    let msg.params.source_fpath = expand('#'.g:clap.start.bufnr.':p')
  endif
  call clap#job#daemon#send_message(json_encode(msg))
endfunction

function! clap#client#send_params(params) abort
  let s:req_id += 1
  let params = a:params
  let params.id = s:req_id
  call clap#job#daemon#send_message(json_encode(params))
endfunction

function! clap#client#handle(msg) abort
  let decoded = json_decode(a:msg)

  " Only process the latest request, drop the outdated responses.
  if s:req_id != decoded.id
    return
  endif

  if has_key(decoded, 'error')
    call clap#helper#echo_error('[daemon_handle] '.decoded.error)
    return
  endif

  if decoded.provider_id ==# 'filer'
    if decoded.event ==# 'on_move'
      call clap#impl#on_move#handle_filer_preview(decoded)
    else
      call clap#provider#filer#daemon_handle(decoded)
    endif
    return
  endif

  if decoded.event ==# 'on_init'
    call g:clap.display.set_lines(decoded.lines)
    return
  endif

  call clap#impl#on_move#handle_file_preview(decoded)
endfunction

let &cpoptions = s:save_cpo
unlet s:save_cpo
