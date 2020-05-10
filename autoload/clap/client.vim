let s:req_id = 0

function! s:client_handle(msg) abort
  let decoded = json_decode(a:msg)
  echom string(decoded)

  if has_key(decoded, 'lines')
    let lines = decoded.lines
    call g:clap.preview.show(lines)
    call g:clap.preview.set_syntax(clap#ext#into_filetype(s:fname))
    call clap#preview#highlight_header()
  endif
endfunction

function! clap#client#create() abort
  let g:client_job_id = clap#job#stdio#start_rpc_service(function('s:client_handle'))
endfunction

function! s:into_filename(line) abort
  if g:clap_enable_icon && clap#maple#is_available()
    return a:line[4:]
  else
    return a:line
  endif
endfunction

function! clap#client#send_message_on_move() abort
  let s:req_id += 1

  let fname = s:into_filename(g:clap.display.getcurline())
  let s:fname = fname
  let msg = json_encode({
        \ 'method': 'client.on_move',
        \ 'params': {'fname': fname, 'cwd': clap#rooter#working_dir(), 'enable_icon': g:clap_enable_icon ? v:true : v:false, 'id': g:clap.provider.id},
        \ 'id': s:req_id
        \ })
  call clap#job#stdio#send_message(msg)
endfunction

