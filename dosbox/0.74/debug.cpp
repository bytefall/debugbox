/*
 *  Copyright (C) 2002-2009  The DOSBox Team
 *
 *  This program is free software; you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation; either version 2 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program; if not, write to the Free Software
 *  Foundation, Inc., 59 Temple Place - Suite 330, Boston, MA 02111-1307, USA.
 */

/* $Id: debug.cpp,v 1.97 2009/04/11 19:49:52 c2woody Exp $ */

#include "dosbox.h"
#if C_DEBUG

#include <string.h>
#include <list>
#include <ctype.h>
#include <fstream>
#include <iomanip>
#include <string>
#include <sstream>
using namespace std;

#include <dbus/dbus.h>

#include "debug.h"
#include "cross.h" //snprintf
#include "cpu.h"
#include "video.h"
#include "pic.h"
#include "mapper.h"
#include "cpu.h"
#include "callback.h"
#include "inout.h"
#include "mixer.h"
#include "timer.h"
#include "paging.h"
#include "support.h"
#include "shell.h"
#include "programs.h"
#include "debug_inc.h"
#include "../cpu/lazyflags.h"
#include "keyboard.h"
#include "setup.h"

#ifdef WIN32
void WIN32_Console();
#else
#include <termios.h>
#include <unistd.h>
static struct termios consolesettings;
#endif


class DEBUG;

DEBUG*	pDebugcom	= 0;
bool	exitLoop	= false;


// Heavy Debugging Vars for logging
#if C_HEAVY_DEBUG
static ofstream 	cpuLogFile;
static bool		cpuLog			= false;
static int		cpuLogCounter	= 0;
static int		cpuLogType		= 1;	// log detail
static bool zeroProtect = false;
bool	logHeavy	= false;
#endif



static struct  {
	Bit32u eax,ebx,ecx,edx,esi,edi,ebp,esp,eip;
} oldregs;

static char curSelectorName[3] = { 0,0,0 };

static Segment oldsegs[6];
static Bitu oldflags,oldcpucpl;
DBGBlock dbg;
static Bitu input_count;
Bitu cycle_count;
static bool debugging;


static Bit16u	dataSeg;
static Bit32u	dataOfs;
static bool		showExtend = true;

DBusConnection* _conn;

/***********/
/* Helpers */
/***********/

Bit32u PhysMakeProt(Bit16u selector, Bit32u offset)
{
	Descriptor desc;
	if (cpu.gdt.GetDescriptor(selector,desc)) return desc.GetBase()+offset;
	return 0;
};

Bit32u GetAddress(Bit16u seg, Bit32u offset)
{
	if (seg==SegValue(cs)) return SegPhys(cs)+offset;
	if (cpu.pmode && !(reg_flags & FLAG_VM)) {
		Descriptor desc;
		if (cpu.gdt.GetDescriptor(seg,desc)) return PhysMakeProt(seg,offset);
	}
	return (seg<<4)+offset;
}

static char empty_sel[] = { ' ',' ',0 };

bool GetDescriptorInfo(char* selname, char* out1, char* out2)
{
	Bitu sel;
	Descriptor desc;

	if (strstr(selname,"cs") || strstr(selname,"CS")) sel = SegValue(cs);
	else if (strstr(selname,"ds") || strstr(selname,"DS")) sel = SegValue(ds);
	else if (strstr(selname,"es") || strstr(selname,"ES")) sel = SegValue(es);
	else if (strstr(selname,"fs") || strstr(selname,"FS")) sel = SegValue(fs);
	else if (strstr(selname,"gs") || strstr(selname,"GS")) sel = SegValue(gs);
	else if (strstr(selname,"ss") || strstr(selname,"SS")) sel = SegValue(ss);
	else {
//		sel = GetHexValue(selname,selname);
		if (*selname==0) selname=empty_sel;
	}
	if (cpu.gdt.GetDescriptor(sel,desc)) {
		switch (desc.Type()) {
			case DESC_TASK_GATE:
				sprintf(out1,"%s: s:%08X type:%02X p",selname,desc.GetSelector(),desc.saved.gate.type);
				sprintf(out2,"    TaskGate   dpl : %01X %1X",desc.saved.gate.dpl,desc.saved.gate.p);
				return true;
			case DESC_LDT:
			case DESC_286_TSS_A:
			case DESC_286_TSS_B:
			case DESC_386_TSS_A:
			case DESC_386_TSS_B:
				sprintf(out1,"%s: b:%08X type:%02X pag",selname,desc.GetBase(),desc.saved.seg.type);
				sprintf(out2,"    l:%08X dpl : %01X %1X%1X%1X",desc.GetLimit(),desc.saved.seg.dpl,desc.saved.seg.p,desc.saved.seg.avl,desc.saved.seg.g);
				return true;
			case DESC_286_CALL_GATE:
			case DESC_386_CALL_GATE:
				sprintf(out1,"%s: s:%08X type:%02X p params: %02X",selname,desc.GetSelector(),desc.saved.gate.type,desc.saved.gate.paramcount);
				sprintf(out2,"    o:%08X dpl : %01X %1X",desc.GetOffset(),desc.saved.gate.dpl,desc.saved.gate.p);
				return true;
			case DESC_286_INT_GATE:
			case DESC_286_TRAP_GATE:
			case DESC_386_INT_GATE:
			case DESC_386_TRAP_GATE:
				sprintf(out1,"%s: s:%08X type:%02X p",selname,desc.GetSelector(),desc.saved.gate.type);
				sprintf(out2,"    o:%08X dpl : %01X %1X",desc.GetOffset(),desc.saved.gate.dpl,desc.saved.gate.p);
				return true;
		}
		sprintf(out1,"%s: b:%08X type:%02X parbg",selname,desc.GetBase(),desc.saved.seg.type);
		sprintf(out2,"    l:%08X dpl : %01X %1X%1X%1X%1X%1X",desc.GetLimit(),desc.saved.seg.dpl,desc.saved.seg.p,desc.saved.seg.avl,desc.saved.seg.r,desc.saved.seg.big,desc.saved.seg.g);
		return true;
	} else {
		strcpy(out1,"                                     ");
		strcpy(out2,"                                     ");
	}
	return false;
};


/********************/
/* Breakpoint stuff */
/********************/

enum EBreakpoint { BKPNT_UNKNOWN, BKPNT_PHYSICAL, BKPNT_INTERRUPT, BKPNT_MEMORY, BKPNT_MEMORY_PROT, BKPNT_MEMORY_LINEAR };

#define BPINT_ALL 0x100

class CBreakpoint
{
public:

	CBreakpoint(void);
	void					SetAddress		(Bit16u seg, Bit32u off)	{ location = GetAddress(seg,off);	type = BKPNT_PHYSICAL; segment = seg; offset = off; };
	void					SetAddress		(PhysPt adr)				{ location = adr;				type = BKPNT_PHYSICAL; };
	void					SetInt			(Bit8u _intNr, Bit16u ah)	{ intNr = _intNr, ahValue = ah; type = BKPNT_INTERRUPT; };
	void					SetOnce			(bool _once)				{ once = _once; };
	void					SetType			(EBreakpoint _type)			{ type = _type; };
	void					SetValue		(Bit8u value)				{ ahValue = value; };

	bool					IsActive		(void)						{ return active; };
	void					Activate		(bool _active);

	EBreakpoint				GetType			(void)						{ return type; };
	bool					GetOnce			(void)						{ return once; };
	PhysPt					GetLocation		(void)						{ if (GetType()!=BKPNT_INTERRUPT)	return location;	else return 0; };
	Bit16u					GetSegment		(void)						{ return segment; };
	Bit32u					GetOffset		(void)						{ return offset; };
	Bit8u					GetIntNr		(void)						{ if (GetType()==BKPNT_INTERRUPT)	return intNr;		else return 0; };
	Bit16u					GetValue		(void)						{ if (GetType()!=BKPNT_PHYSICAL)	return ahValue;		else return 0; };

	// statics
	static CBreakpoint*		AddBreakpoint		(Bit16u seg, Bit32u off, bool once);
	static CBreakpoint*		AddIntBreakpoint	(Bit8u intNum, Bit16u ah, bool once);
	static CBreakpoint*		AddMemBreakpoint	(Bit16u seg, Bit32u off);
	static void				ActivateBreakpoints	(PhysPt adr, bool activate);
	static bool				CheckBreakpoint		(PhysPt adr);
	static bool				CheckBreakpoint		(Bitu seg, Bitu off);
	static bool				CheckIntBreakpoint	(PhysPt adr, Bit8u intNr, Bit16u ahValue);
	static bool				IsBreakpoint		(PhysPt where);
	static bool				IsBreakpointDrawn	(PhysPt where);
	static bool				DeleteBreakpoint	(PhysPt where);
	static bool				DeleteByIndex		(Bit16u index);
	static void				DeleteAll			(void);
	static void				ShowList			(void);


private:
	EBreakpoint	type;
	// Physical
	PhysPt		location;
	Bit8u		oldData;
	Bit16u		segment;
	Bit32u		offset;
	// Int
	Bit8u		intNr;
	Bit16u		ahValue;
	// Shared
	bool		active;
	bool		once;

	static std::list<CBreakpoint*>	BPoints;
public:
	static CBreakpoint*				ignoreOnce;
};

CBreakpoint::CBreakpoint(void):
location(0),
active(false),once(false),
segment(0),offset(0),intNr(0),ahValue(0),
type(BKPNT_UNKNOWN) { };

void CBreakpoint::Activate(bool _active)
{
#if !C_HEAVY_DEBUG
	if (GetType()==BKPNT_PHYSICAL) {
		if (_active) {
			// Set 0xCC and save old value
			Bit8u data = mem_readb(location);
			if (data!=0xCC) {
				oldData = data;
				mem_writeb(location,0xCC);
			};
		} else {
			// Remove 0xCC and set old value
			if (mem_readb (location)==0xCC) {
				mem_writeb(location,oldData);
			};
		}
	}
#endif
	active = _active;
};

// Statics
std::list<CBreakpoint*> CBreakpoint::BPoints;
CBreakpoint*			CBreakpoint::ignoreOnce = 0;
Bitu					ignoreAddressOnce = 0;

CBreakpoint* CBreakpoint::AddBreakpoint(Bit16u seg, Bit32u off, bool once)
{
	CBreakpoint* bp = new CBreakpoint();
	bp->SetAddress		(seg,off);
	bp->SetOnce			(once);
	BPoints.push_front	(bp);
	return bp;
};

CBreakpoint* CBreakpoint::AddIntBreakpoint(Bit8u intNum, Bit16u ah, bool once)
{
	CBreakpoint* bp = new CBreakpoint();
	bp->SetInt			(intNum,ah);
	bp->SetOnce			(once);
	BPoints.push_front	(bp);
	return bp;
};

CBreakpoint* CBreakpoint::AddMemBreakpoint(Bit16u seg, Bit32u off)
{
	CBreakpoint* bp = new CBreakpoint();
	bp->SetAddress		(seg,off);
	bp->SetOnce			(false);
	bp->SetType			(BKPNT_MEMORY);
	BPoints.push_front	(bp);
	return bp;
};

void CBreakpoint::ActivateBreakpoints(PhysPt adr, bool activate)
{
	// activate all breakpoints
	std::list<CBreakpoint*>::iterator i;
	CBreakpoint* bp;
	for(i=BPoints.begin(); i != BPoints.end(); i++) {
		bp = (*i);
		// Do not activate, when bp is an actual adress
		if (activate && (bp->GetType()==BKPNT_PHYSICAL) && (bp->GetLocation()==adr)) {
			// Do not activate :)
			continue;
		}
		bp->Activate(activate);	
	};
};

bool CBreakpoint::CheckBreakpoint(Bitu seg, Bitu off)
// Checks if breakpoint is valid an should stop execution
{
	if ((ignoreAddressOnce!=0) && (GetAddress(seg,off)==ignoreAddressOnce)) {
		ignoreAddressOnce = 0;
		return false;
	} else
		ignoreAddressOnce = 0;

	// Search matching breakpoint
	std::list<CBreakpoint*>::iterator i;
	CBreakpoint* bp;
	for(i=BPoints.begin(); i != BPoints.end(); i++) {
		bp = (*i);
		if ((bp->GetType()==BKPNT_PHYSICAL) && bp->IsActive() && (bp->GetSegment()==seg) && (bp->GetOffset()==off)) {
			// Ignore Once ?
			if (ignoreOnce==bp) {
				ignoreOnce=0;
				bp->Activate(true);
				return false;
			};
			// Found, 
			if (bp->GetOnce()) {
				// delete it, if it should only be used once
				(BPoints.erase)(i);
				bp->Activate(false);
				delete bp;
			} else {
				ignoreOnce = bp;
			};
			return true;
		} 
#if C_HEAVY_DEBUG
		// Memory breakpoint support
		else if (bp->IsActive()) {
			if ((bp->GetType()==BKPNT_MEMORY) || (bp->GetType()==BKPNT_MEMORY_PROT) || (bp->GetType()==BKPNT_MEMORY_LINEAR)) {
				// Watch Protected Mode Memoryonly in pmode
				if (bp->GetType()==BKPNT_MEMORY_PROT) {
					// Check if pmode is active
					if (!cpu.pmode) return false;
					// Check if descriptor is valid
					Descriptor desc;
					if (!cpu.gdt.GetDescriptor(bp->GetSegment(),desc)) return false;
					if (desc.GetLimit()==0) return false;
				}

				Bitu address; 
				if (bp->GetType()==BKPNT_MEMORY_LINEAR) address = bp->GetOffset();
				else address = GetAddress(bp->GetSegment(),bp->GetOffset());
				Bit8u value=0;
				if (mem_readb_checked(address,&value)) return false;
				if (bp->GetValue() != value) {
					// Yup, memory value changed
//					DEBUG_ShowMsg("DEBUG: Memory breakpoint %s: %04X:%04X - %02X -> %02X\n",(bp->GetType()==BKPNT_MEMORY_PROT)?"(Prot)":"",bp->GetSegment(),bp->GetOffset(),bp->GetValue(),value);
					bp->SetValue(value);
					return true;
				};		
			} 		
		};
#endif
	};
	return false;
};

bool CBreakpoint::CheckIntBreakpoint(PhysPt adr, Bit8u intNr, Bit16u ahValue)
// Checks if interrupt breakpoint is valid an should stop execution
{
	if ((ignoreAddressOnce!=0) && (adr==ignoreAddressOnce)) {
		ignoreAddressOnce = 0;
		return false;
	} else
		ignoreAddressOnce = 0;

	// Search matching breakpoint
	std::list<CBreakpoint*>::iterator i;
	CBreakpoint* bp;
	for(i=BPoints.begin(); i != BPoints.end(); i++) {
		bp = (*i);
		if ((bp->GetType()==BKPNT_INTERRUPT) && bp->IsActive() && (bp->GetIntNr()==intNr)) {
			if ((bp->GetValue()==BPINT_ALL) || (bp->GetValue()==ahValue)) {
				// Ignoie it once ?
				if (ignoreOnce==bp) {
					ignoreOnce=0;
					bp->Activate(true);
					return false;
				};
				// Found
				if (bp->GetOnce()) {
					// delete it, if it should only be used once
					(BPoints.erase)(i);
					bp->Activate(false);
					delete bp;
				} else {
					ignoreOnce = bp;
				}
				return true;
			}
		};
	};
	return false;
};

void CBreakpoint::DeleteAll() 
{
	std::list<CBreakpoint*>::iterator i;
	CBreakpoint* bp;
	for(i=BPoints.begin(); i != BPoints.end(); i++) {
		bp = (*i);
		bp->Activate(false);
		delete bp;
	};
	(BPoints.clear)();
};


bool CBreakpoint::DeleteByIndex(Bit16u index) 
{
	// Search matching breakpoint
	int nr = 0;
	std::list<CBreakpoint*>::iterator i;
	CBreakpoint* bp;
	for(i=BPoints.begin(); i != BPoints.end(); i++) {
		if (nr==index) {
			bp = (*i);
			(BPoints.erase)(i);
			bp->Activate(false);
			delete bp;
			return true;
		}
		nr++;
	};
	return false;
};

bool CBreakpoint::DeleteBreakpoint(PhysPt where) 
{
	// Search matching breakpoint
	std::list<CBreakpoint*>::iterator i;
	CBreakpoint* bp;
	for(i=BPoints.begin(); i != BPoints.end(); i++) {
		bp = (*i);
		if ((bp->GetType()==BKPNT_PHYSICAL) && (bp->GetLocation()==where)) {
			(BPoints.erase)(i);
			bp->Activate(false);
			delete bp;
			return true;
		}
	};
	return false;
};

bool CBreakpoint::IsBreakpoint(PhysPt adr) 
// is there a breakpoint at address ?
{
	// Search matching breakpoint
	std::list<CBreakpoint*>::iterator i;
	CBreakpoint* bp;
	for(i=BPoints.begin(); i != BPoints.end(); i++) {
		bp = (*i);
		if ((bp->GetType()==BKPNT_PHYSICAL) && (bp->GetSegment()==adr)) {
			return true;
		};
		if ((bp->GetType()==BKPNT_PHYSICAL) && (bp->GetLocation()==adr)) {
			return true;
		};
	};
	return false;
};

bool CBreakpoint::IsBreakpointDrawn(PhysPt adr) 
// valid breakpoint, that should be drawn ?
{
	// Search matching breakpoint
	std::list<CBreakpoint*>::iterator i;
	CBreakpoint* bp;
	for(i=BPoints.begin(); i != BPoints.end(); i++) {
		bp = (*i);
		if ((bp->GetType()==BKPNT_PHYSICAL) && (bp->GetLocation()==adr)) {
			// Only draw, if breakpoint is not only once, 
			return !bp->GetOnce();
		};
	};
	return false;
};

void CBreakpoint::ShowList(void)
{
	// iterate list 
	/*int nr = 0;
	std::list<CBreakpoint*>::iterator i;
	for(i=BPoints.begin(); i != BPoints.end(); i++) {
		CBreakpoint* bp = (*i);
		if (bp->GetType()==BKPNT_PHYSICAL) {
			DEBUG_ShowMsg("%02X. BP %04X:%04X\n",nr,bp->GetSegment(),bp->GetOffset());
		} else if (bp->GetType()==BKPNT_INTERRUPT) {
			if (bp->GetValue()==BPINT_ALL)	DEBUG_ShowMsg("%02X. BPINT %02X\n",nr,bp->GetIntNr());					
			else							DEBUG_ShowMsg("%02X. BPINT %02X AH=%02X\n",nr,bp->GetIntNr(),bp->GetValue());
		} else if (bp->GetType()==BKPNT_MEMORY) {
			DEBUG_ShowMsg("%02X. BPMEM %04X:%04X (%02X)\n",nr,bp->GetSegment(),bp->GetOffset(),bp->GetValue());
		} else if (bp->GetType()==BKPNT_MEMORY_PROT) {
			DEBUG_ShowMsg("%02X. BPPM %04X:%08X (%02X)\n",nr,bp->GetSegment(),bp->GetOffset(),bp->GetValue());
		} else if (bp->GetType()==BKPNT_MEMORY_LINEAR ) {
			DEBUG_ShowMsg("%02X. BPLM %08X (%02X)\n",nr,bp->GetOffset(),bp->GetValue());
		};
		nr++;
	}*/
};


Bitu _ret = 0;
Bit32u _skipEip = 0;


void break_reply(DBusPendingCall* pcall, void* user_data)
{
	DBusMessage* reply = dbus_pending_call_steal_reply(pcall);

	DBusError err;
	dbus_error_init(&err);

	bool brk = false;

	if(!dbus_message_get_args(reply, &err,
		DBUS_TYPE_BOOLEAN, &brk,
		DBUS_TYPE_INVALID))
	{
//		cout << err.name << err.message;
		dbus_error_free(&err);
	}

	dbus_message_unref(reply);
	dbus_pending_call_unref(pcall);

	if(!brk)
	{
		_skipEip = reg_eip;
/*
		// run:
		debugging = false;
		DOSBOX_SetNormalLoop();
		// run end
*/
//		CPU_Cycles = 0;
	}
}


bool DEBUG_Breakpoint()
{
	if(CBreakpoint::CheckBreakpoint(SegValue(cs), reg_eip))
	{
		CBreakpoint::ActivateBreakpoints(GetAddress(SegValue(cs), reg_eip), false);

		return true;
	}

	if(_skipEip == reg_eip)
	{
		_skipEip = 0;
/*
		exitLoop = false;
		CPU_Cycles = 1;
		CBreakpoint::ignoreOnce = 0;

		Bits r = (*cpudecoder)();

		if(r > 0)
		{
			_ret = (*CallBack_Handlers[r])();

			if(_ret)
			{
				exitLoop = true;
				CPU_Cycles = CPU_CycleLeft = 0;
			}
		}
*/
		return false;
	}

	DBusError err;
	dbus_error_init(&err);

	DBusConnection* session = dbus_bus_get(DBUS_BUS_SESSION, &err);
	DBusMessage* msg = dbus_message_new_method_call("com.dosbox.dbg", "/", "com.dosbox.dbg", "breakNow");

	DBusPendingCall* pcall;
	dbus_connection_send_with_reply(session, msg, &pcall, -1);
	dbus_pending_call_set_notify(pcall, break_reply, NULL, NULL);

	dbus_message_unref(msg);
	dbus_connection_unref(session);

	return true;

/*
	if(brk && _reply)
	{
		DBusMessageIter ri;
		dbus_message_iter_init_append(_reply, &ri);

//		Bit16u cs_ = SegValue(cs);
//		dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &cs_);
		dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_eip);

		if(!dbus_connection_send(_conn, _reply, NULL))
			E_Exit("dbus error: dbus_connection_send failed");

		dbus_connection_flush(_conn);
		dbus_message_unref(_reply);

		_reply = NULL;
	}
*/
};


bool DEBUG_IntBreakpoint(Bit8u intNum)
{/*
	if(_reply)
	{
		DBusMessageIter ri;
		dbus_message_iter_init_append(_reply, &ri);

//		Bit16u cs_ = SegValue(cs);
//		dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &cs_);
		dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_eip);

		if(!dbus_connection_send(_conn, _reply, NULL))
			E_Exit("dbus error: dbus_connection_send failed");

		dbus_connection_flush(_conn);
		dbus_message_unref(_reply);

		_reply = NULL;

		return true;
	}
*/

	/* First get the phyiscal address and check for a set Breakpoint */
	PhysPt where=GetAddress(SegValue(cs),reg_eip);
	if (!CBreakpoint::CheckIntBreakpoint(where,intNum,reg_ah)) return false;
	// Found. Breakpoint is valid
	CBreakpoint::ActivateBreakpoints(where,false);	// Deactivate all breakpoints
	return true;
};


bool DEBUG_ExitLoop(void)
{
	if (exitLoop) {
		exitLoop = false;
		return true;
	}
	return false;
};


Bitu DEBUG_Loop(void)
{
	_ret = 0;

//TODO Disable sound
	GFX_Events();
	// Interrupt started ? - then skip it
	Bit16u oldCS	= SegValue(cs);
	Bit32u oldEIP	= reg_eip;
	PIC_runIRQs();

	dbus_connection_read_write_dispatch(_conn, -1);

	/*SDL_Delay(1);

	if((oldCS!=SegValue(cs)) || (oldEIP!=reg_eip))
	{
		CBreakpoint::AddBreakpoint(oldCS,oldEIP,true);
		CBreakpoint::ActivateBreakpoints(SegPhys(cs)+reg_eip,true);
		debugging=false;
		DOSBOX_SetNormalLoop();
		return 0;
	}

	return DEBUG_CheckKeys();*/

	return _ret;
}

void DEBUG_Enable(bool pressed) {
	if (!pressed)
		return;

	debugging=true;

	DOSBOX_SetLoop(&DEBUG_Loop);

	KEYBOARD_ClrBuffer();
}


// DEBUG.COM stuff

class DEBUG : public Program {
public:
	DEBUG()		{ pDebugcom	= this;	active = false; };
	~DEBUG()	{ pDebugcom	= 0; };

	bool IsActive() { return active; };

	void Run(void)
	{
		if(cmd->FindExist("/NOMOUSE",false)) {
	        	real_writed(0,0x33<<2,0);
			return;
		}
	   
		char filename[128];
		char args[256];
	
		cmd->FindCommand(1,temp_line);
		safe_strncpy(filename,temp_line.c_str(),128);
		// Read commandline
		Bit16u i	=2;
		bool ok		= false; 
		args[0]		= 0;
		for (;cmd->FindCommand(i++,temp_line)==true;) {
			strncat(args,temp_line.c_str(),256);
			strncat(args," ",256);
		}
		// Start new shell and execute prog		
		active = true;
		// Save cpu state....
		Bit16u oldcs	= SegValue(cs);
		Bit32u oldeip	= reg_eip;	
		Bit16u oldss	= SegValue(ss);
		Bit32u oldesp	= reg_esp;

		// Workaround : Allocate Stack Space
		Bit16u segment;
		Bit16u size = 0x200 / 0x10;
		if (DOS_AllocateMemory(&segment,&size)) {
			SegSet16(ss,segment);
			reg_sp = 0x200;
			// Start shell
			DOS_Shell shell;
			shell.Execute(filename,args);
			DOS_FreeMemory(segment);
		}
		// set old reg values
		SegSet16(ss,oldss);
		reg_esp = oldesp;
		SegSet16(cs,oldcs);
		reg_eip = oldeip;
	};

private:
	bool	active;
};

void DEBUG_CheckExecuteBreakpoint(Bit16u seg, Bit32u off)
{
	if (pDebugcom && pDebugcom->IsActive()) {
		CBreakpoint::AddBreakpoint(seg,off,true);		
		CBreakpoint::ActivateBreakpoints(SegPhys(cs)+reg_eip,true);	
		pDebugcom = 0;
	};
};

Bitu DEBUG_EnableDebugger(void)
{
	exitLoop = true;
	DEBUG_Enable(true);
	CPU_Cycles=CPU_CycleLeft=0;
	return 0;
};

static void DEBUG_ProgramStart(Program * * make) {
	*make=new DEBUG;
}

// INIT 

void DEBUG_SetupConsole() {}


static void DEBUG_ShutDown(Section*)
{
	dbus_connection_unref(_conn);

	CBreakpoint::DeleteAll();
}

/*
DBusHandlerResult dbus_filter(DBusConnection* conn, DBusMessage* msg, void* data)
{
	if(dbus_message_is_signal(msg, "com.dosbox", "run"))
	{
		debugging = false;

		CBreakpoint::ActivateBreakpoints(SegPhys(cs) + reg_eip, true);
		ignoreAddressOnce = SegPhys(cs) + reg_eip;

		DOSBOX_SetNormalLoop();

		return DBUS_HANDLER_RESULT_HANDLED;
	}

	return DBUS_HANDLER_RESULT_NOT_YET_HANDLED;
}
*/

void unregister_handler(DBusConnection* conn, void* data)
{
}


DBusHandlerResult dbg_handler(DBusConnection* conn, DBusMessage* msg, void* data)
{
	DBusHandlerResult ret = DBUS_HANDLER_RESULT_NOT_YET_HANDLED;

	DBusError err;
	dbus_error_init(&err);

	if(dbus_message_is_method_call(msg, "com.dosbox", "attach"))
	{
		char* service = NULL;

		if(!dbus_message_get_args(msg, &err,
			DBUS_TYPE_STRING, &service,
			DBUS_TYPE_INVALID))
		{
//			cout << err.name << err.message;
			dbus_error_free(&err);
		}
		else
		{
		}

		ret = DBUS_HANDLER_RESULT_HANDLED;
	}
	else if(dbus_message_is_method_call(msg, "com.dosbox", "detach"))
	{
		ret = DBUS_HANDLER_RESULT_HANDLED;
	}

	return ret;
}


DBusHandlerResult cpu_handler(DBusConnection* conn, DBusMessage* msg, void* data)
{
	DBusHandlerResult ret = DBUS_HANDLER_RESULT_NOT_YET_HANDLED;

	if(dbus_message_is_method_call(msg, "com.dosbox", "get"))
	{
		DBusMessage* rm = dbus_message_new_method_return(msg);
		DBusMessageIter ri;
		dbus_message_iter_init_append(rm, &ri);

		dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &cpu.pmode);
		dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &cpu.code.big);

		if(!dbus_connection_send(conn, rm, NULL))
			E_Exit("dbus error: dbus_connection_send failed");

		dbus_connection_flush(conn);
		dbus_message_unref(rm);

		ret = DBUS_HANDLER_RESULT_HANDLED;
	}
	else if(dbus_message_is_method_call(msg, "com.dosbox", "callback_info"))
	{
		DBusError err;
		dbus_error_init(&err);

		uint16_t index;

		if(!dbus_message_get_args(msg, &err,
			DBUS_TYPE_UINT16, &index,
			DBUS_TYPE_INVALID))
		{
			LOG_MSG("dbus error: %s", err.name, err.message);
			dbus_error_free(&err);
		}
		else
		{
			DBusMessage* rm = dbus_message_new_method_return(msg);
			DBusMessageIter ri;
			dbus_message_iter_init_append(rm, &ri);

			const char* info = CALLBACK_GetDescription(index);

			if(info)
				dbus_message_iter_append_basic(&ri, DBUS_TYPE_STRING, &info);

			if(!dbus_connection_send(conn, rm, NULL))
				E_Exit("dbus error: dbus_connection_send failed");

			dbus_connection_flush(conn);
			dbus_message_unref(rm);

			ret = DBUS_HANDLER_RESULT_HANDLED;
		}
	}
	else
	{
		if(dbus_message_is_method_call(msg, "com.dosbox", "step_in"))
		{
			exitLoop = false;
			CPU_Cycles = 1;
			CBreakpoint::ignoreOnce = 0;

			Bits r = (*cpudecoder)();

			if(r > 0)
			{
				_ret = (*CallBack_Handlers[r])();

				if(_ret)
				{
					exitLoop = true;
					CPU_Cycles = CPU_CycleLeft = 0;
				}
			}

			ret = DBUS_HANDLER_RESULT_HANDLED;
		}
		else if(dbus_message_is_method_call(msg, "com.dosbox", "run"))
		{
/*
			debugging = false;

			CBreakpoint::ActivateBreakpoints(SegPhys(cs) + reg_eip, true);
			ignoreAddressOnce = SegPhys(cs) + reg_eip;

			DOSBOX_SetNormalLoop();

			ret = DBUS_HANDLER_RESULT_HANDLED;
*/
			debugging = false;
			DOSBOX_SetNormalLoop();

			ret = DBUS_HANDLER_RESULT_HANDLED;
		}

		DBusMessage* rm = dbus_message_new_method_return(msg);
		DBusMessageIter ri;
		dbus_message_iter_init_append(rm, &ri);

		dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_eip);

		if(!dbus_connection_send(conn, rm, NULL))
			E_Exit("dbus error: dbus_connection_send failed");

		dbus_connection_flush(conn);
		dbus_message_unref(rm);
	}

	return ret;
}


DBusHandlerResult cpu_regs_handler(DBusConnection* conn, DBusMessage* msg, void* data)
{
	char** path;
	dbus_message_get_path_decomposed(msg, &path);

	DBusHandlerResult ret = DBUS_HANDLER_RESULT_NOT_YET_HANDLED;

	if(dbus_message_is_method_call(msg, "com.dosbox", "get"))
	{
		DBusMessage* rm = dbus_message_new_method_return(msg);
		DBusMessageIter ri;
		dbus_message_iter_init_append(rm, &ri);

		if(path[2])
		{
			std::string reg(path[2]);

			if(reg.length() == 3)
			{
				if(reg == "eax")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_eax);
				else if(reg == "ebx")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_ebx);
				else if(reg == "ecx")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_ecx);
				else if(reg == "edx")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_edx);
				else if(reg == "esi")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_esi);
				else if(reg == "edi")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_edi);
				else if(reg == "ebp")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_ebp);
				else if(reg == "esp")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_esp);
				else if(reg == "eip")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_eip);
				else if(reg == "efl")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_flags);
			}
			else
			{
				Bit16u cs_ = SegValue(cs);
				Bit16u ds_ = SegValue(ds);
				Bit16u es_ = SegValue(es);
				Bit16u fs_ = SegValue(fs);
				Bit16u gs_ = SegValue(gs);
				Bit16u ss_ = SegValue(ss);

				if(reg == "cs")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &cs_);
				else if(reg == "ds")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &ds_);
				else if(reg == "es")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &es_);
				else if(reg == "fs")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &fs_);
				else if(reg == "gs")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &gs_);
				else if(reg == "ss")
					dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &ss_);
			}
		}
		else
		{
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_eax);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_ebx);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_ecx);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_edx);

			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_esi);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_edi);

			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_ebp);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_esp);

			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &reg_eip);

			Bit16u cs_ = SegValue(cs);
			Bit16u ds_ = SegValue(ds);
			Bit16u es_ = SegValue(es);
			Bit16u fs_ = SegValue(fs);
			Bit16u gs_ = SegValue(gs);
			Bit16u ss_ = SegValue(ss);

			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &cs_);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &ds_);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &es_);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &fs_);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &gs_);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &ss_);

			bool f;

			f = GETFLAG(OF);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(DF);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(IF);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(SF);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(ZF);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(AF);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(PF);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(CF);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);

			f = GETFLAG(TF);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(IOPL) >> 12;
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(NT);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(VM);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(AC);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
			f = GETFLAG(ID);
			dbus_message_iter_append_basic(&ri, DBUS_TYPE_BOOLEAN, &f);
		}

		if(!dbus_connection_send(conn, rm, NULL))
			E_Exit("dbus error: dbus_connection_send failed");

		dbus_connection_flush(conn);
		dbus_message_unref(rm);

		ret = DBUS_HANDLER_RESULT_HANDLED;
	}

	dbus_free_string_array(path);

	return ret;
}


DBusHandlerResult mem_handler(DBusConnection* conn, DBusMessage* msg, void* data)
{
	DBusHandlerResult ret = DBUS_HANDLER_RESULT_NOT_YET_HANDLED;

	if(dbus_message_is_method_call(msg, "com.dosbox", "get"))
	{
		DBusError err;
		dbus_error_init(&err);

		uint16_t segment;
		uint32_t offset;
		uint32_t length;

		if(dbus_message_get_args(msg, &err,
			DBUS_TYPE_UINT16, &segment,
			DBUS_TYPE_UINT32, &offset,
			DBUS_TYPE_UINT32, &length,
			DBUS_TYPE_INVALID))
		{
			DBusMessage* rm = dbus_message_new_method_return(msg);
			DBusMessageIter ri;
			dbus_message_iter_init_append(rm, &ri);

			DBusMessageIter bytes;
			dbus_message_iter_open_container(&ri, DBUS_TYPE_ARRAY, DBUS_TYPE_BYTE_AS_STRING, &bytes);
/*
			PhysPt start = GetAddress(segment, offset);
			PhysPt end = GetAddress(segment, length);

			for(int i = start; i < end; ++i)
			{
				uint8_t b;
				if(mem_readb_checked(i, &b)) b = 0;

				dbus_message_iter_append_basic(&bytes, DBUS_TYPE_BYTE, &b);
			}
*/
			for(uint32_t i = 0; i < length; ++i)
			{
				uint8_t b;
				if(mem_readb_checked(GetAddress(segment, offset + i), &b)) b = 0;

				dbus_message_iter_append_basic(&bytes, DBUS_TYPE_BYTE, &b);
			}

			dbus_message_iter_close_container(&ri, &bytes);

			if(!dbus_connection_send(conn, rm, NULL))
				E_Exit("dbus error: dbus_connection_send failed");

			dbus_connection_flush(conn);
			dbus_message_unref(rm);

			ret = DBUS_HANDLER_RESULT_HANDLED;
		}
		else
			dbus_error_free(&err);
	}
	else if(dbus_message_is_method_call(msg, "com.dosbox", "set"))
	{
		DBusError err;
		dbus_error_init(&err);

		uint16_t segment;
		uint32_t offset;

		if(dbus_message_has_signature(msg,
			DBUS_TYPE_UINT16_AS_STRING
			DBUS_TYPE_UINT32_AS_STRING
			DBUS_TYPE_BYTE_AS_STRING))
		{
			uint8_t b;

			if(!dbus_message_get_args(msg, &err,
				DBUS_TYPE_UINT16, &segment,
				DBUS_TYPE_UINT32, &offset,
				DBUS_TYPE_BYTE, &b,
				DBUS_TYPE_INVALID))
			{
//				cout << err.name << err.message;
				dbus_error_free(&err);
			}
			else
			{
				DBusMessage* rm = dbus_message_new_method_return(msg);
				DBusMessageIter ri;
				dbus_message_iter_init_append(rm, &ri);

				PhysPt addr = GetAddress(segment, offset);

				uint8_t tmp;
				if(mem_readb_checked(addr, &tmp)) tmp = 0;

				dbus_message_iter_append_basic(&ri, DBUS_TYPE_BYTE, &tmp);

				mem_writeb(addr, b);

				if(!dbus_connection_send(conn, rm, NULL))
					E_Exit("dbus error: dbus_connection_send failed");

				dbus_connection_flush(conn);
				dbus_message_unref(rm);

				ret = DBUS_HANDLER_RESULT_HANDLED;
			}
		}
		else if(dbus_message_has_signature(msg,
			DBUS_TYPE_UINT16_AS_STRING
			DBUS_TYPE_UINT32_AS_STRING
			DBUS_TYPE_UINT16_AS_STRING))
		{
			uint16_t w;

			if(!dbus_message_get_args(msg, &err,
				DBUS_TYPE_UINT16, &segment,
				DBUS_TYPE_UINT32, &offset,
				DBUS_TYPE_UINT16, &w,
				DBUS_TYPE_INVALID))
			{
//				cout << err.name << err.message;
				dbus_error_free(&err);
			}
			else
			{
				DBusMessage* rm = dbus_message_new_method_return(msg);
				DBusMessageIter ri;
				dbus_message_iter_init_append(rm, &ri);

				PhysPt addr = GetAddress(segment, offset);

				uint16_t tmp;
				if(mem_readw_checked(addr, &tmp)) tmp = 0;

				dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT16, &tmp);

				mem_writew(addr, w);

				if(!dbus_connection_send(conn, rm, NULL))
					E_Exit("dbus error: dbus_connection_send failed");

				dbus_connection_flush(conn);
				dbus_message_unref(rm);

				ret = DBUS_HANDLER_RESULT_HANDLED;
			}
		}
		else if(dbus_message_has_signature(msg,
			DBUS_TYPE_UINT16_AS_STRING
			DBUS_TYPE_UINT32_AS_STRING
			DBUS_TYPE_UINT32_AS_STRING))
		{
			uint32_t d;

			if(!dbus_message_get_args(msg, &err,
				DBUS_TYPE_UINT16, &segment,
				DBUS_TYPE_UINT32, &offset,
				DBUS_TYPE_UINT32, &d,
				DBUS_TYPE_INVALID))
			{
//				cout << err.name << err.message;
				dbus_error_free(&err);
			}
			else
			{
				DBusMessage* rm = dbus_message_new_method_return(msg);
				DBusMessageIter ri;
				dbus_message_iter_init_append(rm, &ri);

				PhysPt addr = GetAddress(segment, offset);

				uint32_t tmp;
				if(mem_readd_checked(addr, &tmp)) tmp = 0;

				dbus_message_iter_append_basic(&ri, DBUS_TYPE_UINT32, &tmp);

				mem_writed(addr, d);

				if(!dbus_connection_send(conn, rm, NULL))
					E_Exit("dbus error: dbus_connection_send failed");

				dbus_connection_flush(conn);
				dbus_message_unref(rm);

				ret = DBUS_HANDLER_RESULT_HANDLED;
			}
		}
		else if(dbus_message_has_signature(msg,
			DBUS_TYPE_UINT16_AS_STRING
			DBUS_TYPE_UINT32_AS_STRING
			DBUS_TYPE_ARRAY_AS_STRING
			DBUS_TYPE_BYTE_AS_STRING))
		{
			uint8_t* bytes;
			int bytes_len;

			if(!dbus_message_get_args(msg, &err,
				DBUS_TYPE_UINT16, &segment,
				DBUS_TYPE_UINT32, &offset,
				DBUS_TYPE_ARRAY, DBUS_TYPE_BYTE, &bytes, &bytes_len,
				DBUS_TYPE_INVALID))
			{
//				cout << err.name << err.message;
				dbus_error_free(&err);
			}
			else
			{
				DBusMessage* rm = dbus_message_new_method_return(msg);
				DBusMessageIter ri;
				dbus_message_iter_init_append(rm, &ri);
				dbus_message_iter_append_basic(&ri, DBUS_TYPE_BYTE, &bytes_len);

				while(bytes_len-- > 0)
					mem_writeb(GetAddress(segment, offset++), *bytes++);

				if(!dbus_connection_send(conn, rm, NULL))
					E_Exit("dbus error: dbus_connection_send failed");

				dbus_connection_flush(conn);
				dbus_message_unref(rm);

				ret = DBUS_HANDLER_RESULT_HANDLED;
			}
		}
	}

	return ret;
}

Bitu debugCallback;


void DEBUG_Init(Section* sec)
{
	DBusError err;
	dbus_error_init(&err);

	_conn = dbus_bus_get(DBUS_BUS_SESSION, &err);

	if(dbus_error_is_set(&err))
	{
		LOG_MSG("dbus error: %s", err.name, err.message);
		dbus_error_free(&err);
		E_Exit("dbus error!");
	}

	int ret = dbus_bus_request_name(_conn, "com.dosbox", DBUS_NAME_FLAG_REPLACE_EXISTING , &err);

	if(dbus_error_is_set(&err))
	{
		LOG_MSG("dbus error: %s", err.name, err.message);
		dbus_error_free(&err);
		E_Exit("dbus error!");
	}

	DBusObjectPathVTable dbg_h = { &unregister_handler, &dbg_handler };
	DBusObjectPathVTable cpu_h = { &unregister_handler, &cpu_handler };
	DBusObjectPathVTable reg_h = { &unregister_handler, &cpu_regs_handler };
	DBusObjectPathVTable mem_h = { &unregister_handler, &mem_handler };

	dbus_connection_register_object_path(_conn, "/dbg", &dbg_h, NULL);
	dbus_connection_register_object_path(_conn, "/cpu", &cpu_h, NULL);
	dbus_connection_register_fallback(_conn, "/cpu/regs", &reg_h, NULL);
	dbus_connection_register_fallback(_conn, "/mem", &mem_h, NULL);

//	dbus_connection_add_filter(_conn, dbus_filter, NULL, NULL);


	MSG_Add("DEBUG_CONFIGFILE_HELP","Debugger related options.\n");

	/* Add some keyhandlers */
	MAPPER_AddHandler(DEBUG_Enable,MK_pause,MMOD2,"debugger","Debugger");

	/* setup debug.com */
	PROGRAMS_MakeFile("DEBUG.COM",DEBUG_ProgramStart);

	/* Setup callback */
	debugCallback=CALLBACK_Allocate();
	CALLBACK_Setup(debugCallback,DEBUG_EnableDebugger,CB_RETF,"debugger");
	/* shutdown function */
	sec->AddDestroyFunction(&DEBUG_ShutDown);
}


void DEBUG_ShowMsg(char const* format,...)
{
}


void LOG::operator()(char const* format, ...)
{
}


void LOG_StartUp()
{
}


// HEAVY DEBUGGING STUFF

#if C_HEAVY_DEBUG

void DEBUG_HeavyWriteLogInstruction()
{
};


bool DEBUG_HeavyIsBreakpoint()
{
	return false;
}

#endif // HEAVY DEBUG


#endif // DEBUG


