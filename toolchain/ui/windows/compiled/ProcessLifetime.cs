using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

namespace Sovra.Desktop
{
    // Install before spawning any child. Descendants inherit membership, not the
    // non-inheritable job handle. Windows closes that handle when this host exits.
    internal static class ProcessLifetime
    {
        private static IntPtr lifetimeJob;

        [StructLayout(LayoutKind.Sequential)]
        private struct BasicLimits
        {
            internal long ProcessTime, JobTime;
            internal uint Flags;
            internal UIntPtr MinimumWorkingSet, MaximumWorkingSet;
            internal uint ActiveProcesses;
            internal UIntPtr Affinity;
            internal uint Priority, Scheduling;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct IoCounters
        {
            internal ulong ReadOperations, WriteOperations, OtherOperations;
            internal ulong ReadBytes, WriteBytes, OtherBytes;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct ExtendedLimits
        {
            internal BasicLimits Basic;
            internal IoCounters Io;
            internal UIntPtr ProcessMemory, JobMemory, PeakProcessMemory, PeakJobMemory;
        }

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr CreateJobObject(IntPtr attributes, string name);
        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetInformationJobObject(IntPtr job, int kind, ref ExtendedLimits limits, uint size);
        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
        [DllImport("kernel32.dll")]
        private static extern IntPtr GetCurrentProcess();
        [DllImport("kernel32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CloseHandle(IntPtr handle);

        internal static void ProtectChildren()
        {
            if (lifetimeJob != IntPtr.Zero) return;
            IntPtr job = CreateJobObject(IntPtr.Zero, null);
            if (job == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error(), "Cannot create desktop process lifetime guard.");
            var limits = new ExtendedLimits();
            limits.Basic.Flags = 0x2000; // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE; no breakaway.
            if (!SetInformationJobObject(job, 9, ref limits, (uint)Marshal.SizeOf(typeof(ExtendedLimits))) ||
                !AssignProcessToJobObject(job, GetCurrentProcess()))
            {
                int error = Marshal.GetLastWin32Error();
                CloseHandle(job);
                throw new Win32Exception(error, "Cannot protect desktop child processes.");
            }
            // Intentionally process-lifetime ownership: closing or finalizing this
            // handle here would kill the host itself. Only the OS closes it on exit.
            lifetimeJob = job;
        }
    }
}
