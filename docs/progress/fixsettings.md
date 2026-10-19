  ### Executive Summary                                                                    
                                                                                           
  The freeze when opening the Settings menu in DOOM Eternal under PeekGame is not caused by
  a deadlock or stalled I/O in the Bink video player, nor by a blocked Winsock call.       
                                                                                           
  Instead, the UI is gated on an asynchronous WinHTTP PlayFab inventory synchronization.   
  Because PeekGame’s WinHTTP layer successfully connects and returns HTTP 200 for PlayFab  
  endpoints (E6125.playfabapi.com), the game enters an online synchronization state.       
  However, the game fails to properly consume the online inventory payload into its        
  internal Warehouse state machine (evidenced by Warehouse: Don't know how to handle item  
  of class '' that came from service and WARNING: idHUD_Reticle::SetActiveReticle: no SWF  
  name found for reticle style unknown). The Settings menu displays the modal "NOTICE /    
  Loading…" dialog while continuously retrying PlayFab requests every ~10 seconds.         
                                                                                           
  On a genuine offline machine, WinHTTP requests fail immediately with                     
  ERROR_WINHTTP_CANNOT_CONNECT (12029), prompting the engine to bypass online inventory    
  synchronization and load the campaign Settings menu immediately.                         
  ──────                                                                                   
  ### Key Evidence & Analysis by Category                                                  
                                                                                           
  #### 1. Winsock Activity vs. Blocking                                                    
                                                                                           
  • Status: Not blocking / Idle.                                                           
  • Details:                                                                               
      • Winsock is initialized at startup via WSAStartup (doom.log: Winsock Initialized).  
      • Thread 2 (LWP 48944, "wsa-overlapped") runs the PeekGame reactor in                
      game_imports::api::network::overlapped::reactor, waiting in __GI___poll with         
      timeout=-1 on 2 file descriptors.                                                    
      • No game worker thread or main thread was blocked in any Winsock API (connect, send,
      recv, select, WSAWaitForMultipleEvents).                                             
      • Winsock is not used for the Settings or Warehouse flows.                           
                                                                                           
                                                                                           
  #### 2. WinHTTP Activity & The Settings Gate                                             
                                                                                           
  • Status: Causal trigger of the modal loading hang.                                      
  • Details:                                                                               
      • When opening Settings, DOOMEternalx64vk.exe issues asynchronous WinHTTP requests   
      to:                                                                                  
          • POST E6125.playfabapi.com:443/Group/ListMembershipOpportunities                
          • POST E6125.playfabapi.com:443/Event/WriteTelemetryEvents                       
          • POST E6125.playfabapi.com:443/Client/GetUserInventory                          
      • In logs/doom.log (lines 10606–12690), these requests succeed with HTTP 200 via host
      libcurl, headers are read, and STATUS_DATA_AVAILABLE / STATUS_READ_COMPLETE callbacks
      execute on worker thread LWP 48900.                                                  
      • Because online requests succeed, the engine treats the session as online and gates 
      the Settings UI behind the modal "NOTICE / Loading…".                                
      • The returned payload does not resolve the player profile items, yielding:          
        WARNING: idHUD_Reticle::SetActiveReticle: no SWF name found for reticle style      
      unknown                                                                              
                                                                                           
      • The game retries the request every 10–11 seconds (e.g. timestamps 07:12:01,        
      07:12:11, 07:12:21 GMT) indefinitely, keeping the modal dialog displayed.            
                                                                                           
                                                                                           
  #### 3. Bink Video Playback (tutorial_weapon_wheel.bk2)                                  
                                                                                           
  • Status: Incidental, not causal.                                                        
  • Details:                                                                               
      • The asset exists on disk at /home/nobus/.steam/debian-                             
      installation/steamapps/common/DOOMEternal/base/video/tutorials/tutorial_weapon_wheel.
      bk2 (1,999,484 bytes).                                                               
      • Disassembly of DOOMEternalx64vk.exe at 0x141cc8a00–0x141cc8a50 reveals             
      idFile_Bink::WaitForBufferToComplete:                                                
        0x141cc8a00: cmp    %r13d,(%rbx)                                                   
        0x141cc8a03: je     0x141cc8a51         ; buffer ready -> skip stall log           
        0x141cc8a11: lea    0x11ea258(%rip),%rcx ; "idFile_Bink::WaitForBufferToComplete   
      stall [%s]\n"                                                                        
        0x141cc8a18: call   0x140363590         ; log warning                              
        0x141cc8a23: call   0x1403e4de0         ; EnterCriticalSection                     
        0x141cc8a3e: call   0x1403e4a30         ; SleepConditionVariableCS (*0x142a1b520)  
        0x141cc8a4c: call   0x1403e4e20         ; LeaveCriticalSection                     
                                                                                           
      • The stall message is a non-fatal warning emitted when a video begins before its    
      ring buffer has prefetched sufficiently. Boot_Sequence.bk2 emits the exact same stall
      message at line 311 of doom.log and proceeds immediately.                            
      • In GDB backtraces, both Bink threads (Thread 13 / LWP 48887 "BinkAsy0" and Thread  
      12 / LWP 48888 "BinkAsy1") were idle inside bink2w64.dll!0x18002ee15 calling         
      game_imports::api::threading::srw::sleep_condition_cs(). Neither was stuck in file   
      I/O or buffer waits.                                                                 
                                                                                           
                                                                                           
  #### 4. Main Thread & Render Loop State                                                  
                                                                                           
  • Status: Active UI pump, not deadlocked.                                                
  • Call Stack (Thread 1 / LWP 48810):                                                     
      • Frame 0: syscall() (Linux futex wait)                                              
      • Frame 2: game_imports::api::threading::objects::wait_single(handle=0x67000810,     
      ms=0xffffffff)                                                                       
      • Frame 4: DOOMEternalx64vk.exe+0x3e4f2a (WaitForSingleObject)                       
      • Frame 10: DOOMEternalx64vk.exe+0x1875030 (idTech 7 job sync)                       
      • Frame 18: DOOMEternalx64vk.exe+0x43d9eb in idCommonLocal::Frame()                  
  • Diagnostic: The handle 0x67000810 is an Object::Semaphore. The main thread completes   
  rendering each frame and waits on the frame job semaphore. The renderer and window       
  message pump are alive and rendering the modal "Loading..." screen continuously.         
  ──────                                                                                   
  ### Resolution & Test Plan                                                               
                                                                                           
  #### The Fix: Game Quirk ForceHttpOffline                                                
  
  A dedicated, reversible quirk is available in mod.rs and winhttp.rs:583-604:
  
  • Mechanism: Quirk::ForceHttpOffline forces winhttp::http_offline() to return            
  ERROR_WINHTTP_CANNOT_CONNECT (12029) for doometernalx64vk.
  • Behavior: The game detects that network services are down immediately upon opening     
  Settings, abandons the PlayFab inventory sync attempt, and falls back to standard offline
  campaign Settings.
  
  #### Runtime Control Switch
  
  • Default: Automatically active for doometernalx64vk.
  • Force Online (Debug): PEEKGAME_HTTP_OFFLINE=0
  • Force Offline (Any game): PEEKGAME_HTTP_OFFLINE=1
  • Trace HTTP status: PEEKGAME_TRACE_HTTP=1
  
  #### Verification
  
  1. Quirks unit tests:
    cargo test --offline quirks
  
  2. Runtime comparison:
      • With PEEKGAME_HTTP_OFFLINE=0 (Baseline/Failing): Opening Settings logs repeated    
      POST ...playfabapi.com:443 returning HTTP 200, emitting SetActiveReticle: no SWF name
      found for reticle style unknown and locking the UI in "NOTICE / Loading…".           
      • With Default / PEEKGAME_HTTP_OFFLINE=1 (Fixed): Log shows [http]
      PEEKGAME_HTTP_OFFLINE: ... -> CANNOT_CONNECT. The Settings menu opens without the    
      modal hang.

