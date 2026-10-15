/* Windows x64 structured exception handling: vectored handlers, a hardware fault
 * resumed by a handler, a frame's language handler found through .pdata followed
 * by RtlUnwindEx to a landing pad, function tables registered at run time, and the
 * unhandled-exception filter. Exit 0 passes; other codes name the check. */
#include <windows.h>
static volatile LONG vectored_calls, unwind_calls, filter_calls;
extern void fault_resume(void);
extern int do_fault(void);
extern int guarded(void);
extern void guarded_landing(void);
__asm__(
    ".text\n.globl do_fault\ndo_fault:\n"
    " xor %eax,%eax\n mov $0x10,%rcx\n movl $1,(%rcx)\n"
    ".globl fault_resume\nfault_resume:\n ret\n"
    // A function with a frame handler: raises through raise_it, lands after unwind.
    ".globl guarded\n.def guarded; .scl 2; .type 32; .endef\n.seh_proc guarded\nguarded:\n"
    " push %rbx\n .seh_pushreg %rbx\n sub $32,%rsp\n .seh_stackalloc 32\n .seh_endprologue\n"
    " .seh_handler frame_handler, @except, @unwind\n"
    " mov $0x1234,%ebx\n call raise_it\n xor %eax,%eax\n jmp 1f\n"
    ".globl guarded_landing\nguarded_landing:\n"
    " cmp $0x1234,%ebx\n je 1f\n mov $-1,%eax\n"
    "1: add $32,%rsp\n pop %rbx\n ret\n.seh_endproc\n");
__attribute__((noinline)) void raise_it(void){
 ULONG_PTR args[1]={5};
 __asm__ volatile("mov $0x5678,%%ebx":::"rbx"); /* the unwind restores guarded's rbx */
 RaiseException(0xE0000002,0,1,args);
}
EXCEPTION_DISPOSITION frame_handler(EXCEPTION_RECORD *rec,void *frame,CONTEXT *ctx,DISPATCHER_CONTEXT *dc){
 (void)dc;
 if(rec->ExceptionFlags&EXCEPTION_UNWINDING){unwind_calls++;return ExceptionContinueSearch;}
 if(rec->ExceptionCode!=0xE0000002||rec->NumberParameters!=1||rec->ExceptionInformation[0]!=5)return ExceptionContinueSearch;
 RtlUnwindEx(frame,(void*)guarded_landing,rec,(void*)77,ctx,0);
 return ExceptionContinueSearch;
}
static LONG WINAPI vectored(EXCEPTION_POINTERS *p){
 EXCEPTION_RECORD *r=p->ExceptionRecord;
 if(r->ExceptionCode==0xE0000001&&r->NumberParameters==1&&r->ExceptionInformation[0]==7){vectored_calls++;return EXCEPTION_CONTINUE_EXECUTION;}
 if(r->ExceptionCode==EXCEPTION_ACCESS_VIOLATION&&r->ExceptionInformation[0]==1&&r->ExceptionInformation[1]==0x10){
  p->ContextRecord->Rip=(DWORD64)fault_resume;p->ContextRecord->Rax=99;return EXCEPTION_CONTINUE_EXECUTION;}
 return EXCEPTION_CONTINUE_SEARCH;
}
static LONG WINAPI filter(EXCEPTION_POINTERS *p){
 if(p->ExceptionRecord->ExceptionCode!=0xE0000003)return EXCEPTION_CONTINUE_SEARCH;
 filter_calls++;return EXCEPTION_CONTINUE_EXECUTION;
}
void entry(void){
 void *h=AddVectoredExceptionHandler(1,vectored);
 if(!h)ExitProcess(1);
 ULONG_PTR seven=7;RaiseException(0xE0000001,0,1,&seven);
 if(vectored_calls!=1)ExitProcess(2);
 if(do_fault()!=99)ExitProcess(3);
 if(!RemoveVectoredExceptionHandler(h)||RemoveVectoredExceptionHandler(h))ExitProcess(4);
 if(guarded()!=77)ExitProcess(5);
 if(unwind_calls!=1)ExitProcess(6);
 DWORD64 base=0;
 RUNTIME_FUNCTION *own=RtlLookupFunctionEntry((DWORD64)guarded+4,&base,0);
 if(!own||base!=(DWORD64)GetModuleHandleW(0)||base+own->BeginAddress!=(DWORD64)guarded)ExitProcess(7);
 static RUNTIME_FUNCTION table[2]={{0x100,0x180,0x1000},{0x200,0x240,0x1010}};
 DWORD64 region=0x5a000000;
 if(RtlLookupFunctionEntry(region+0x210,&base,0))ExitProcess(8);
 if(!RtlAddFunctionTable(table,2,region))ExitProcess(9);
 if(RtlLookupFunctionEntry(region+0x210,&base,0)!=&table[1]||base!=region)ExitProcess(10);
 if(RtlLookupFunctionEntry(region+0x1a0,&base,0))ExitProcess(11);
 if(!RtlDeleteFunctionTable(table)||RtlLookupFunctionEntry(region+0x210,&base,0))ExitProcess(12);
 SetUnhandledExceptionFilter(filter);
 RaiseException(0xE0000003,0,0,0);
 if(filter_calls!=1)ExitProcess(13);
 ExitProcess(0);
}
