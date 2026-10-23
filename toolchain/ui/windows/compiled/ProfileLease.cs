using System;
using System.IO;
using System.Threading;

namespace Sovra.Desktop
{
    // Cooperating desktop hosts own one profile for their entire lifetime.
    // The OS releases this handle on crash; the empty marker is never deleted.
    internal sealed class ProfileLease : IDisposable
    {
        private FileStream handle;
        private ProfileLease(FileStream owned) { handle = owned; }

        internal static ProfileLease Acquire(string directory)
        {
            string marker = Path.Combine(Path.GetFullPath(directory), ".desktop.lock");
            try
            {
                return new ProfileLease(new FileStream(marker, FileMode.OpenOrCreate,
                    FileAccess.ReadWrite, FileShare.None));
            }
            catch (IOException error)
            {
                int code = error.HResult & 65535;
                if (code == 32 || code == 33)
                    throw new IOException("This application profile is already open in another desktop instance. " +
                        "Close that instance before opening this profile again.", error);
                throw;
            }
        }

        public void Dispose()
        {
            FileStream owned = Interlocked.Exchange(ref handle, null);
            if (owned != null) owned.Dispose();
        }
    }
}
